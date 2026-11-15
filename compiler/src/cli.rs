use std::ffi::OsString;

pub const USAGE: &str =
    "usage: aleph <check|fmt|test|run|build|explain|ast|tokens> [OPTIONS] FILE [ARGS..]";

const COMMANDS: &[&str] = &[
    "check", "fmt", "test", "run", "build", "explain", "ast", "tokens",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub command: String,
    pub file: String,
    pub json: bool,
    pub write: bool,
    pub out: Option<String>,
    pub args: Vec<String>,
}

/// Parse arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut args = args
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned());

    let command = args.next().ok_or_else(|| "missing command".to_string())?;
    if !COMMANDS.contains(&command.as_str()) {
        return Err(format!("unknown command: {command}"));
    }

    let tokens: Vec<String> = args.collect();
    let mut file = None;
    let mut json = false;
    let mut write = false;
    let mut out = None;
    let mut program_args = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let arg = &tokens[i];
        if arg == "--json" {
            json = true;
            i += 1;
        } else if arg == "-w" {
            write = true;
            i += 1;
        } else if arg == "-o" {
            let value = tokens
                .get(i + 1)
                .ok_or_else(|| "-o requires a value".to_string())?;
            out = Some(value.clone());
            i += 2;
        } else if arg.starts_with('-') {
            return Err(format!("unknown option: {arg}"));
        } else if file.is_none() {
            file = Some(arg.clone());
            i += 1;
            if command == "run" {
                program_args = tokens[i..].to_vec();
                break;
            }
        } else {
            return Err(format!("unexpected argument: {arg}"));
        }
    }

    let file = file.ok_or_else(|| "missing file".to_string())?;

    Ok(Cli {
        command,
        file,
        json,
        write,
        out,
        args: program_args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Cli, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn parses_options_before_file() {
        let cli = parse_args(&["check", "--json", "foo.aph"]).unwrap();
        assert_eq!(cli.command, "check");
        assert_eq!(cli.file, "foo.aph");
        assert!(cli.json);
        assert!(!cli.write);
        assert_eq!(cli.out, None);
        assert!(cli.args.is_empty());
    }

    #[test]
    fn parses_options_after_file() {
        let cli = parse_args(&["build", "foo.aph", "-o", "out", "-w"]).unwrap();
        assert_eq!(cli.file, "foo.aph");
        assert_eq!(cli.out.as_deref(), Some("out"));
        assert!(cli.write);
    }

    #[test]
    fn run_program_args_are_not_interpreted() {
        let cli = parse_args(&["run", "foo.aph", "--json", "-w", "-o", "out"]).unwrap();
        assert_eq!(cli.file, "foo.aph");
        assert_eq!(cli.args, ["--json", "-w", "-o", "out"]);
        assert!(!cli.json);
        assert!(!cli.write);
        assert_eq!(cli.out, None);
    }

    #[test]
    fn unknown_option_is_an_error() {
        assert!(parse_args(&["check", "--bogus", "foo.aph"]).is_err());
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(parse_args(&["check", "--json"]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_argument_is_lossy_not_a_panic() {
        use std::os::unix::ffi::OsStringExt;

        let cli = parse(vec![
            OsString::from("check"),
            OsString::from_vec(vec![b'f', b'o', 0x80]),
        ])
        .unwrap();
        assert_eq!(cli.file, "fo\u{fffd}");
    }
}
