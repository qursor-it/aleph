use std::fs;
use std::process::ExitCode;

use alephc::cli;
use alephc::diag::Diagnostic;
use alephc::lexer;

fn main() -> ExitCode {
    let cli = match cli::parse(std::env::args_os().skip(1)) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };

    let diagnostics = match cli.command.as_str() {
        "check" => check(&cli),
        // Later phases are still stubbed.
        _ => vec![Diagnostic::not_implemented(&cli.file)],
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = Diagnostic::emit_all(&diagnostics, &mut out) {
        eprintln!("aleph: failed to write diagnostic: {err}");
        return ExitCode::from(2);
    }
    ExitCode::from(if diagnostics.is_empty() { 0 } else { 1 })
}

/// Minimal `aleph check`: read the file, lex it, and stop at the first phase
/// that is not implemented yet (ADR-0015).
fn check(cli: &cli::Cli) -> Vec<Diagnostic> {
    let source = match fs::read_to_string(&cli.file) {
        Ok(source) => source,
        Err(_) => {
            return vec![Diagnostic::error(
                "X002",
                cli.file.as_str(),
                0,
                0,
                "cannot read file",
            )]
        }
    };

    let (_lines, diagnostics) = lexer::lex(&cli.file, &source);
    if diagnostics.is_empty() {
        vec![Diagnostic::not_implemented(&cli.file)]
    } else {
        diagnostics
    }
}
