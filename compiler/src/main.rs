use std::fs;
use std::io::Write;
use std::process::ExitCode;

use alephc::ast;
use alephc::caps;
use alephc::check as type_check;
use alephc::cli;
use alephc::diag::Diagnostic;
use alephc::examples;
use alephc::explain;
use alephc::interp;
use alephc::parser;
use alephc::printer;

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
        "explain" => explain_command(&cli),
        "ast" => ast_command(&cli),
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
    ExitCode::from(if diagnostics.is_empty() { 0 } else { 1 })
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

/// `aleph check`: lex and parse, reject non-canonical text (F001), type check,
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
    if source != canonical {
        return vec![canonical_diagnostic(&cli.file, &source, &canonical)];
    }

    let checked = type_check::check(&cli.file, &file);
    if !checked.diagnostics.is_empty() {
        return checked.diagnostics;
    }

    let capabilities = caps::check_caps(&cli.file, &file);
    if !capabilities.is_empty() {
        return capabilities;
    }

    examples::check(&cli.file, &file)
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
    Diagnostic::error("F001", file, line, col, "not in canonical form").with_canonical(canonical)
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
        // Write a sibling temp file, then rename over the original so a failed
        // write cannot leave a half-written source file.
        let temp = format!("{}.aleph-tmp", cli.file);
        if let Err(err) = fs::write(&temp, canonical) {
            eprintln!("aleph: failed to write file: {err}");
            return ExitCode::from(2);
        }
        if let Err(err) = fs::rename(&temp, &cli.file) {
            let _ = fs::remove_file(&temp);
            eprintln!("aleph: failed to replace file: {err}");
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
