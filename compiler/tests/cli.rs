use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aleph"))
        .args(args)
        .output()
        .expect("failed to run aleph")
}

/// Write a temporary source file and return its path.
fn temp_source(name: &str, contents: &str) -> String {
    let path = std::env::temp_dir().join(format!("aleph-test-{}-{name}", std::process::id()));
    std::fs::write(&path, contents).expect("write temporary source");
    path.to_string_lossy().into_owned()
}

const LEXABLE_SOURCE: &str = "fn main -> unit\n nil\n";

#[test]
fn no_args_is_usage_error() {
    let out = run(&[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn unknown_subcommand_is_usage_error() {
    let out = run(&["bogus", "foo.aph"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn missing_file_is_usage_error() {
    let out = run(&["check"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn check_reports_not_implemented() {
    let file = temp_source("check_x001.aph", LEXABLE_SOURCE);
    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X001");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["file"], file.as_str());
    assert_eq!(value["line"], 0);
    assert_eq!(value["col"], 0);
    assert_eq!(value["msg"], "not implemented");
}

#[test]
fn check_accepts_json_option_before_file() {
    let file = temp_source("check_json.aph", LEXABLE_SOURCE);
    let out = run(&["check", "--json", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X001");
    assert_eq!(value["file"], file.as_str());
}

#[test]
fn check_missing_file_reports_x002() {
    let file = std::env::temp_dir().join(format!("aleph-missing-{}.aph", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let file = file.to_string_lossy().into_owned();

    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X002");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["file"], file.as_str());
    assert_eq!(value["line"], 0);
    assert_eq!(value["col"], 0);
    assert_eq!(value["msg"], "cannot read file");
}

#[test]
fn check_reports_lexer_diagnostics_as_json_lines() {
    let file = temp_source("check_lex_error.aph", "fn main -> unit\n Foo\n");
    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());

    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout}");
    let value: serde_json::Value =
        serde_json::from_str(lines[0]).expect("diagnostic must be valid JSON");
    assert_eq!(value["code"], "L002");
    assert_eq!(value["file"], file.as_str());
    assert_eq!(value["line"], 2);
    assert_eq!(value["col"], 2);
    assert_eq!(value["msg"], "invalid token");
}

#[test]
fn unknown_option_is_usage_error() {
    let out = run(&["check", "--bogus", "foo.aph"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn run_program_args_after_file_are_not_interpreted() {
    let out = run(&["run", "foo.aph", "--json", "-w", "-o", "out"]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X001");
    assert_eq!(value["file"], "foo.aph");
}

#[test]
fn every_stubbed_subcommand_reports_not_implemented() {
    for command in ["check", "fmt", "test", "run", "build", "explain", "tokens"] {
        let file = if command == "check" {
            temp_source("check_all.aph", LEXABLE_SOURCE)
        } else {
            "foo.aph".to_string()
        };
        let out = run(&[command, file.as_str()]);
        assert_eq!(out.status.code(), Some(1), "command {command}");

        let value: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
        assert_eq!(value["code"], "X001", "command {command}");
        assert_eq!(value["file"], file.as_str(), "command {command}");
    }
}

#[test]
fn ast_prints_the_parsed_tree_as_json() {
    let file = temp_source("ast_ok.aph", LEXABLE_SOURCE);
    let out = run(&["ast", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["id"], "n0");
    assert_eq!(value["fns"][0]["name"], "main");
    assert_eq!(value["fns"][0]["result"]["kind"], "nil");
}

#[test]
fn ast_reports_parse_diagnostics() {
    let file = temp_source("ast_error.aph", "fn main -> unit\n nope\n");
    let out = run(&["ast", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());

    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout}");
    let value: serde_json::Value =
        serde_json::from_str(lines[0]).expect("diagnostic must be valid JSON");
    assert_eq!(value["code"], "P003");
    assert_eq!(value["file"], file.as_str());
    assert_eq!(value["found"], "nope");
}

#[test]
fn ast_missing_file_reports_x002() {
    let file = std::env::temp_dir().join(format!("aleph-ast-missing-{}.aph", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let file = file.to_string_lossy().into_owned();

    let out = run(&["ast", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X002");
    assert_eq!(value["msg"], "cannot read file");
}

#[test]
fn check_reports_parser_diagnostics() {
    let file = temp_source("check_parse_error.aph", "fn main -> unit\n nope\n");
    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "P003");
    assert_eq!(value["found"], "nope");
}
