use std::process::ExitCode;

use alephc::cli;
use alephc::diag::Diagnostic;

fn main() -> ExitCode {
    let cli = match cli::parse(std::env::args_os().skip(1)) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };

    let diagnostic = Diagnostic::not_implemented(&cli.file);
    match serde_json::to_string(&diagnostic) {
        Ok(line) => println!("{line}"),
        Err(err) => {
            eprintln!("aleph: failed to serialize diagnostic: {err}");
            return ExitCode::from(2);
        }
    }
    ExitCode::from(1)
}
