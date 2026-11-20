use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use alephc::ast;
use alephc::caps;
use alephc::cgen;
use alephc::check as type_check;
use alephc::cli;
use alephc::diag::Diagnostic;
use alephc::edit;
use alephc::examples;
use alephc::explain;
use alephc::interp;
use alephc::llms;
use alephc::mcp;
use alephc::parser;
use alephc::printer;
use alephc::tokens;

fn main() -> ExitCode {
    let cli = match cli::parse(std::env::args_os().skip(1)) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };

    match cli.command.as_str() {
        "check" => emit_diagnostics(&check(&cli)),
        "fmt" => fmt_command(&cli),
        "test" => test_command(&cli),
        "run" => run_command(&cli),
        "build" => build_command(&cli),
        "explain" => explain_command(&cli),
        "ast" => ast_command(&cli),
        "edit" => edit_command(&cli),
        "llms" => llms_command(),
        "mcp" => mcp_command(),
        "tokens" => tokens_command(&cli),
        // Later phases are still stubbed.
        _ => emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]),
    }
}

fn emit_diagnostics(diagnostics: &[Diagnostic]) -> ExitCode {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = Diagnostic::emit_all(diagnostics, &mut out) {
        eprintln!("aleph: failed to write diagnostic: {err}");
        return ExitCode::from(2);
    }
    ExitCode::from(u8::from(diagnostics.iter().any(Diagnostic::is_error)))
}

fn read_source(cli: &cli::Cli) -> Result<String, Box<Diagnostic>> {
    fs::read_to_string(&cli.file).map_err(|_| {
        Box::new(Diagnostic::error(
            "X002",
            cli.file.as_str(),
            0,
            0,
            "cannot read file",
        ))
    })
}

/// `aleph check`: lex and parse, flag non-canonical text (F001: a warning that
/// goes on, an error that stops with `--strict`, ADR-0040), type check,
/// capability check, then run inline examples (E001/E002).
fn check(cli: &cli::Cli) -> Vec<Diagnostic> {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return vec![*diagnostic],
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return output.diagnostics;
    }

    let Some(file) = output.file else {
        return vec![Diagnostic::not_implemented(&cli.file)];
    };

    let canonical = printer::print_file(&file);
    let mut diagnostics = Vec::new();
    if source != canonical {
        let diagnostic = canonical_diagnostic(&cli.file, &source, &canonical);
        if cli.strict {
            return vec![diagnostic];
        }
        // As a warning F001 carries no `fix`: a whole-file replace would
        // conflict with every other fix (`fmt`/`edit` canonicalize anyway).
        diagnostics.push(diagnostic.into_warning().without_fix());
    }

    let checked = type_check::check(&cli.file, &file);
    if !checked.diagnostics.is_empty() {
        diagnostics.extend(checked.diagnostics);
        return diagnostics;
    }

    let capabilities = caps::check_caps(&cli.file, &file);
    if !capabilities.is_empty() {
        diagnostics.extend(capabilities);
        return diagnostics;
    }

    diagnostics.extend(examples::check(&cli.file, &file));
    diagnostics
}

/// `aleph test`: lex, parse, type check and capability check (no F001), then
/// run the inline examples only (E002).
fn test_command(cli: &cli::Cli) -> ExitCode {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return emit_diagnostics(&output.diagnostics);
    }

    let Some(file) = output.file else {
        return emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]);
    };

    let checked = type_check::check(&cli.file, &file);
    if !checked.diagnostics.is_empty() {
        return emit_diagnostics(&checked.diagnostics);
    }

    let capabilities = caps::check_caps(&cli.file, &file);
    if !capabilities.is_empty() {
        return emit_diagnostics(&capabilities);
    }

    emit_diagnostics(&examples::test(&cli.file, &file))
}

/// `aleph run`: lex, parse, type check and capability check (no F001), then
/// execute `main` with the trailing program arguments.
fn run_command(cli: &cli::Cli) -> ExitCode {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return emit_diagnostics(&output.diagnostics);
    }

    let Some(file) = output.file else {
        return emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]);
    };

    let checked = type_check::check(&cli.file, &file);
    if !checked.diagnostics.is_empty() {
        return emit_diagnostics(&checked.diagnostics);
    }

    let capabilities = caps::check_caps(&cli.file, &file);
    if !capabilities.is_empty() {
        return emit_diagnostics(&capabilities);
    }

    if !file.fns.iter().any(|function| function.name == "main") {
        return emit_diagnostics(&[Diagnostic::error(
            "X003",
            cli.file.as_str(),
            0,
            0,
            "no main function",
        )]);
    }

    match interp::run_main(&file, cli.args.clone()) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(trap) => {
            let _ = std::io::stdout().flush();
            let stderr = std::io::stderr();
            let mut err = stderr.lock();
            let _ = writeln!(err, "{}", trap.to_diagnostic(&cli.file).to_json_line());
            let _ = err.flush();
            ExitCode::from(70)
        }
    }
}

/// `aleph build`: check (no F001, no examples), generate C99, and compile it
/// together with the embedded Tlön runtime using the system C compiler.
fn build_command(cli: &cli::Cli) -> ExitCode {
    let Some(out) = cli.out.as_deref() else {
        eprintln!("aleph: build requires -o OUT");
        eprintln!("{}", cli::USAGE);
        return ExitCode::from(2);
    };

    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return emit_diagnostics(&output.diagnostics);
    }

    let Some(file) = output.file else {
        return emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]);
    };

    let checked = type_check::check(&cli.file, &file);
    if !checked.diagnostics.is_empty() {
        return emit_diagnostics(&checked.diagnostics);
    }

    let capabilities = caps::check_caps(&cli.file, &file);
    if !capabilities.is_empty() {
        return emit_diagnostics(&capabilities);
    }

    if !file.fns.iter().any(|function| function.name == "main") {
        return emit_diagnostics(&[Diagnostic::error(
            "X003",
            cli.file.as_str(),
            0,
            0,
            "no main function",
        )]);
    }

    let generated = match cgen::emit(&cli.file, &file, &checked) {
        Ok(generated) => generated,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let directory = match TempDir::new() {
        Ok(directory) => directory,
        Err(error) => {
            eprintln!("aleph: cannot create temporary directory: {error}");
            return ExitCode::from(2);
        }
    };
    let gen_c = directory.path.join("gen.c");
    let tlon_c = directory.path.join("tlon.c");
    let tlon_h = directory.path.join("tlon.h");
    for (path, contents) in [
        (&gen_c, generated.as_str()),
        (&tlon_c, TLON_C),
        (&tlon_h, TLON_H),
    ] {
        if let Err(error) = fs::write(path, contents) {
            eprintln!("aleph: cannot write {}: {error}", path.display());
            return ExitCode::from(2);
        }
    }

    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    // Extra flags, whitespace-separated, after the fixed ones so they win
    // (e.g. `-O0`). cc's stderr is discarded on success, so warning flags
    // only matter with `-Werror` (tests/differential.rs adds
    // `-Wall -Wextra -Werror`).
    let extra = std::env::var("ALEPH_CFLAGS").unwrap_or_default();
    let result = Command::new(&compiler)
        .args(["-std=c99", "-O2", "-pthread"])
        .args(extra.split_whitespace())
        .arg("-o")
        .arg(out)
        .arg(&gen_c)
        .arg(&tlon_c)
        .output();

    match result {
        Ok(output) if output.status.success() => ExitCode::SUCCESS,
        Ok(output) => {
            let stderr = std::io::stderr();
            let mut err = stderr.lock();
            let _ = err.write_all(&output.stderr);
            let _ = err.flush();
            emit_diagnostics(&[Diagnostic::error(
                "X004",
                cli.file.as_str(),
                0,
                0,
                "c compiler failed",
            )])
        }
        Err(error) => {
            eprintln!("aleph: cannot run the C compiler: {error}");
            emit_diagnostics(&[Diagnostic::error(
                "X004",
                cli.file.as_str(),
                0,
                0,
                "c compiler failed",
            )])
        }
    }
}

const TLON_C: &str = include_str!("../../tlon/tlon.c");
const TLON_H: &str = include_str!("../../tlon/tlon.h");

/// A fresh directory under the system temporary directory, removed on drop.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> std::io::Result<Self> {
        let pid = std::process::id();
        let mut counter = 0u32;
        loop {
            let path = std::env::temp_dir().join(format!("aleph-build-{pid}-{counter}"));
            match create_private_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => counter += 1,
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

/// `aleph explain`: lex, parse, type check and capability check (no F001, no
/// examples), then print the deterministic report as text or JSON.
fn explain_command(cli: &cli::Cli) -> ExitCode {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return emit_diagnostics(&output.diagnostics);
    }

    let Some(file) = output.file else {
        return emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]);
    };

    let checked = type_check::check(&cli.file, &file);
    if !checked.diagnostics.is_empty() {
        return emit_diagnostics(&checked.diagnostics);
    }

    let capabilities = caps::check_caps(&cli.file, &file);
    if !capabilities.is_empty() {
        return emit_diagnostics(&capabilities);
    }

    let report = explain::report(&cli.file, &file);
    let text = if cli.json {
        explain::to_json(&report)
    } else {
        explain::to_text(&report)
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = out.write_all(text.as_bytes()) {
        eprintln!("aleph: failed to write explain output: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// F001: the source is not the canonical printing of its AST (SPEC §10).
fn canonical_diagnostic(file: &str, source: &str, canonical: &str) -> Diagnostic {
    let (line, col) = first_difference(source, canonical);
    Diagnostic::error("F001", file, line, col, "not in canonical form")
        .with_fix("replace", "n0", canonical)
        .with_canonical(canonical)
}

/// The 1-based `(line, byte column)` where `source` and `canonical` first
/// differ.
///
/// Extra lines are reported at the first line that exists in only one text
/// (column 1). If every line matches and the texts differ only in the final LF,
/// report the end of the last line; identical text reports `(1, 1)`.
fn first_difference(source: &str, canonical: &str) -> (u32, u32) {
    if source == canonical {
        return (1, 1);
    }

    let source_lines: Vec<&str> = source.lines().collect();
    let canonical_lines: Vec<&str> = canonical.lines().collect();

    for (index, (left, right)) in source_lines.iter().zip(&canonical_lines).enumerate() {
        if left != right {
            return (index as u32 + 1, first_differing_col(left, right));
        }
    }

    if source_lines.len() != canonical_lines.len() {
        return (source_lines.len().min(canonical_lines.len()) as u32 + 1, 1);
    }

    let last_col = source_lines
        .last()
        .map(|line| line.len() as u32 + 1)
        .unwrap_or(1);
    (source_lines.len().max(1) as u32, last_col)
}

/// First differing byte column of two lines, 1-based (SPEC §9 columns are
/// byte columns).
fn first_differing_col(left: &str, right: &str) -> u32 {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut index = 0;
    while index < left.len() && index < right.len() && left[index] == right[index] {
        index += 1;
    }
    index as u32 + 1
}

/// `aleph fmt`: print the canonical text, or write it with `-w`.
fn fmt_command(cli: &cli::Cli) -> ExitCode {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return emit_diagnostics(&output.diagnostics);
    }

    let Some(file) = output.file else {
        return emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]);
    };
    let canonical = printer::print_file(&file);

    if cli.write {
        if let Err(err) = write_in_place(&cli.file, &canonical) {
            eprintln!("aleph: failed to write file: {err}");
            return ExitCode::from(2);
        }
        return ExitCode::SUCCESS;
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = out.write_all(canonical.as_bytes()) {
        eprintln!("aleph: failed to write canonical text: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// Replace `file` with `text` without ever leaving a half-written source.
///
/// The text goes to a unique sibling temp file that inherits the target's
/// permissions, is synced, and is then renamed over the target. A symlink is
/// resolved first so the link itself survives and its target is updated. The
/// temp file is removed on every failure after it was created.
fn write_in_place(file: &str, text: &str) -> std::io::Result<()> {
    let target = match fs::symlink_metadata(file) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(file)?,
        _ => PathBuf::from(file),
    };
    let permissions = fs::metadata(&target)?.permissions();
    let (temp, mut handle) = create_temp_file(&target)?;

    if let Err(err) = fs::set_permissions(&temp, permissions) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }

    let written = handle
        .write_all(text.as_bytes())
        .and_then(|()| handle.sync_all());
    drop(handle);
    if let Err(err) = written {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }

    if let Err(err) = fs::rename(&temp, &target) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

/// Create `<target>.aleph-tmp-<pid>-<n>` with `create_new`, retrying with the
/// next `n` when the name already exists.
fn create_temp_file(target: &Path) -> std::io::Result<(PathBuf, fs::File)> {
    let pid = std::process::id();
    let mut counter = 0u32;
    loop {
        let mut name = target.as_os_str().to_os_string();
        name.push(format!(".aleph-tmp-{pid}-{counter}"));
        let temp = PathBuf::from(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(handle) => return Ok((temp, handle)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => counter += 1,
            Err(err) => return Err(err),
        }
    }
}

/// `aleph edit FILE --json EDITS [-w]`: apply node-level edits (ADR-0029).
///
/// Prints the edited canonical text, or with `-w` writes it and then reports
/// `aleph check` diagnostics for the new file (ids refer to the new AST).
fn edit_command(cli: &cli::Cli) -> ExitCode {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };
    let text = match edit::apply(&cli.file, &source, cli.edits.as_deref().unwrap_or_default()) {
        Ok(text) => text,
        Err(diagnostics) => return emit_diagnostics(&diagnostics),
    };

    if cli.write {
        if let Err(err) = write_in_place(&cli.file, &text) {
            eprintln!("aleph: failed to write file: {err}");
            return ExitCode::from(2);
        }
        return emit_diagnostics(&check(cli));
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = out.write_all(text.as_bytes()) {
        eprintln!("aleph: failed to write edited text: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// `aleph tokens FILE [--tokenizer NAME|all] [--json]` (ADR-0031).
fn tokens_command(cli: &cli::Cli) -> ExitCode {
    let Some(selected) = tokens::select(cli.tokenizer.as_deref()) else {
        eprintln!(
            "unknown tokenizer: {} (known: {}, all)",
            cli.tokenizer.as_deref().unwrap_or_default(),
            tokens::names()
        );
        return ExitCode::from(2);
    };
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let counts: Vec<_> = selected
        .iter()
        .map(|tokenizer| (tokenizer, (tokenizer.count)(&source)))
        .collect();
    // Only the lexer-based counter can fail. Asked for alone, report why;
    // among several, its count is null and the others still count.
    if let [(_, None)] = counts.as_slice() {
        let (_, diagnostics) = alephc::lexer::lex(&cli.file, &source);
        return emit_diagnostics(&diagnostics);
    }

    let text = if cli.json {
        let entries: Vec<serde_json::Value> = counts
            .iter()
            .map(|(tokenizer, count)| {
                serde_json::json!({
                    "tokenizer": tokenizer.name,
                    "method": tokenizer.method,
                    "count": count,
                })
            })
            .collect();
        format!(
            "{}\n",
            serde_json::json!({"file": cli.file, "tokens": entries})
        )
    } else {
        counts
            .iter()
            .map(|(tokenizer, count)| match count {
                Some(count) => format!("{} {count}\n", tokenizer.name),
                None => format!("{} -\n", tokenizer.name),
            })
            .collect()
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = out.write_all(text.as_bytes()) {
        eprintln!("aleph: failed to write token counts: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// `aleph mcp`: serve MCP over stdio until end of input (ADR-0030).
fn mcp_command() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match mcp::serve(stdin.lock(), stdout.lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("aleph: mcp i/o error: {err}");
            ExitCode::from(2)
        }
    }
}

/// `aleph llms`: print the compact spec for LLM context (ADR-0026).
fn llms_command() -> ExitCode {
    let text = llms::render();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = out.write_all(text.as_bytes()) {
        eprintln!("aleph: failed to write llms output: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// `aleph ast`: parse and print the AST as JSON.
fn ast_command(cli: &cli::Cli) -> ExitCode {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return emit_diagnostics(&[*diagnostic]),
    };

    let output = parser::parse(&cli.file, &source);
    if !output.diagnostics.is_empty() {
        return emit_diagnostics(&output.diagnostics);
    }

    let Some(file) = output.file else {
        return emit_diagnostics(&[Diagnostic::not_implemented(&cli.file)]);
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = out.write_all(ast::to_json(&file).as_bytes()) {
        eprintln!("aleph: failed to write AST: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}
