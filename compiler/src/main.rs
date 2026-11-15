use std::fs;
use std::io::Write;
use std::process::ExitCode;

use alephc::ast;
use alephc::cli;
use alephc::diag::Diagnostic;
use alephc::parser;

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

/// `aleph check`: lex and parse, then stop at the first phase that is not
/// implemented yet (ADR-0015).
fn check(cli: &cli::Cli) -> Vec<Diagnostic> {
    let source = match read_source(cli) {
        Ok(source) => source,
        Err(diagnostic) => return vec![*diagnostic],
    };

    let output = parser::parse(&cli.file, &source);
    if output.diagnostics.is_empty() {
        vec![Diagnostic::not_implemented(&cli.file)]
    } else {
        output.diagnostics
    }
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
