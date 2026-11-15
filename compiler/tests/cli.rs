use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aleph"))
        .args(args)
        .output()
        .expect("failed to run aleph")
}

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
    let out = run(&["check", "foo.aph"]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X001");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["file"], "foo.aph");
    assert_eq!(value["line"], 0);
    assert_eq!(value["col"], 0);
    assert_eq!(value["msg"], "not implemented");
}

#[test]
fn check_accepts_json_option_before_file() {
    let out = run(&["check", "--json", "foo.aph"]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X001");
    assert_eq!(value["file"], "foo.aph");
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
fn every_subcommand_reports_not_implemented() {
    for command in [
        "check", "fmt", "test", "run", "build", "explain", "ast", "tokens",
    ] {
        let out = run(&[command, "foo.aph"]);
        assert_eq!(out.status.code(), Some(1), "command {command}");

        let value: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
        assert_eq!(value["code"], "X001", "command {command}");
    }
}
