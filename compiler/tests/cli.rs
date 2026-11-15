use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

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
fn check_accepts_a_clean_file() {
    let file = temp_source("check_clean.aph", LEXABLE_SOURCE);
    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
}

#[test]
fn check_accepts_json_option_before_file() {
    let file = temp_source("check_json.aph", LEXABLE_SOURCE);
    let out = run(&["check", "--json", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
}

#[test]
fn check_reports_e002_for_a_failing_example() {
    let file = temp_source(
        "check_e002.aph",
        "fn twice n:i64 -> i64\n ex twice 2 = 5\n * n 2\nfn main -> unit\n nil\n",
    );
    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "E002");
    assert_eq!(value["expected"], "5");
    assert_eq!(value["found"], "4");
}

#[test]
fn examples_deny_filesystem_access() {
    let repo = repo_root();
    let target = repo.join("tests/aph/out-hermetic.txt");
    let _ = std::fs::remove_file(&target);

    let out = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .current_dir(&repo)
        .args(["check", "tests/aph/ex_fs_denied.aph"])
        .output()
        .expect("failed to run aleph");

    let created = target.exists();
    let _ = std::fs::remove_file(&target);

    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "E002");
    assert_eq!(value["found"], "trap R003");
    assert!(
        !created,
        "example wrote {} despite deny_fs",
        target.display()
    );
}

#[test]
fn test_runs_examples_without_requiring_them() {
    let file = temp_source(
        "test_clean.aph",
        "fn identity n:i64 -> i64\n n\nfn main -> unit\n nil\n",
    );
    let out = run(&["test", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());

    let file = temp_source(
        "test_e002.aph",
        "fn twice n:i64 -> i64\n ex twice 2 = 5\n * n 2\nfn main -> unit\n nil\n",
    );
    let out = run(&["test", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "E002");
    assert_eq!(value["expected"], "5");
    assert_eq!(value["found"], "4");
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
    let file = temp_source("run_args.aph", "fn main -> unit\n print show args\n");
    let out = run(&["run", file.as_str(), "--json", "-w", "-o", "out"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "[\"--json\" \"-w\" \"-o\" \"out\"]\n"
    );
    assert!(out.stderr.is_empty());
}

#[test]
fn run_executes_main_and_streams_output() {
    let file = temp_source(
        "run_ok.aph",
        "fn main -> unit\n do print \"out\"\n do eprint \"err\"\n nil\n",
    );
    let out = run(&["run", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "out\n");
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "err\n");
}

#[test]
fn run_without_main_reports_x003() {
    let file = temp_source("run_no_main.aph", "fn helper -> i64\n 1\n");
    let out = run(&["run", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X003");
    assert_eq!(value["msg"], "no main function");
    assert_eq!(value["file"], file.as_str());
}

#[test]
fn run_does_not_require_canonical_source() {
    let file = temp_source("run_noncanonical.aph", "fn  main -> unit\n\n nil   \n");
    let out = run(&["run", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
}

#[test]
fn run_main_returning_err_exits_one() {
    let file = temp_source(
        "run_main_err.aph",
        "fn main -> res unit str\n err \"bad\"\n",
    );
    let out = run(&["run", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "bad\n");
}

#[test]
fn run_trap_is_json_on_stderr_and_exits_seventy() {
    let file = temp_source("run_trap.aph", "fn main -> unit\n print show / 1 0\n");
    let out = run(&["run", file.as_str()]);
    assert_eq!(out.status.code(), Some(70));
    assert!(out.stdout.is_empty());

    let value: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("stderr must be valid JSON");
    assert_eq!(value["code"], "R002");
    assert_eq!(value["msg"], "division by zero");
    assert_eq!(value["file"], file.as_str());
}

#[test]
fn every_stubbed_subcommand_reports_not_implemented() {
    for command in ["build", "tokens"] {
        let file = "foo.aph".to_string();
        let out = run(&[command, file.as_str()]);
        assert_eq!(out.status.code(), Some(1), "command {command}");

        let value: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
        assert_eq!(value["code"], "X001", "command {command}");
        assert_eq!(value["file"], file.as_str(), "command {command}");
    }
}

#[test]
fn explain_prints_type_diagnostics_and_is_byte_identical_across_runs() {
    let file = temp_source(
        "explain_ill_typed.aph",
        "fn main -> unit\n let x + 1 \"x\"\n nil\n",
    );

    let first = run(&["explain", file.as_str()]);
    let second = run(&["explain", file.as_str()]);
    assert_eq!(first.status.code(), Some(1));
    assert!(first.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout, "explain output must be stable");
    assert_eq!(first.status.code(), second.status.code());

    let stdout = String::from_utf8(first.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout}");
    let value: serde_json::Value =
        serde_json::from_str(lines[0]).expect("diagnostic must be valid JSON");
    assert_eq!(value["code"], "T001");
    assert_eq!(value["file"], file.as_str());

    let file = temp_source(
        "explain_stable.aph",
        "type point x:f64 y:f64\nfn main -> unit\n nil\n",
    );
    let first = run(&["explain", file.as_str()]);
    let second = run(&["explain", file.as_str()]);
    assert_eq!(first.status.code(), Some(0));
    assert!(first.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout, "explain output must be stable");
}

#[test]
fn fmt_prints_canonical_text_and_writes_in_place() {
    let file = temp_source("fmt.aph", "fn  main -> unit\n\n nil   \n");
    let canonical = "fn main -> unit\n nil\n";

    let out = run(&["fmt", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), canonical);

    let out = run(&["fmt", "-w", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), canonical);
    assert!(!std::path::Path::new(&format!("{file}.aleph-tmp")).exists());
}

/// Temp files created next to `file` by `aleph fmt -w`.
fn temp_sibling_files(file: &str) -> Vec<PathBuf> {
    let path = Path::new(file);
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = format!("{}.aleph-tmp-", path.file_name().unwrap().to_string_lossy());
    std::fs::read_dir(directory)
        .expect("read temp directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect()
}

#[test]
fn fmt_write_leaves_no_temp_file() {
    let file = temp_source("fmt_no_temp.aph", "fn  main -> unit\n\n nil   \n");
    let out = run(&["fmt", "-w", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn main -> unit\n nil\n"
    );
    assert_eq!(temp_sibling_files(&file), Vec::<PathBuf>::new());
}

#[cfg(unix)]
#[test]
fn fmt_write_preserves_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let file = temp_source("fmt_permissions.aph", "fn  main -> unit\n\n nil   \n");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
        .expect("set 0600 permissions");

    let out = run(&["fmt", "-w", file.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    assert_eq!(
        std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn main -> unit\n nil\n"
    );
}

#[cfg(unix)]
#[test]
fn fmt_write_follows_a_symlink_and_preserves_it() {
    use std::os::unix::fs::symlink;

    let target = temp_source("fmt_symlink_target.aph", "fn  main -> unit\n\n nil   \n");
    let link =
        std::env::temp_dir().join(format!("aleph-test-{}-fmt_symlink.aph", std::process::id()));
    let _ = std::fs::remove_file(&link);
    symlink(&target, &link).expect("create symlink");
    let link = link.to_string_lossy().into_owned();

    let out = run(&["fmt", "-w", link.as_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());

    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(std::fs::read_link(&link).unwrap(), PathBuf::from(&target));
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "fn main -> unit\n nil\n"
    );
    assert_eq!(temp_sibling_files(&target), Vec::<PathBuf>::new());

    std::fs::remove_file(&link).unwrap();
}

#[test]
fn fmt_reports_parse_diagnostics() {
    let file = temp_source("fmt_error.aph", "fn main -> unit\n nope\n");
    let out = run(&["fmt", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "P003");
    assert_eq!(value["found"], "nope");
}

#[test]
fn check_reports_f001_for_non_canonical_source() {
    let file = temp_source("check_f001.aph", "fn  main -> unit\n\n nil   \n");
    let out = run(&["check", file.as_str()]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "F001");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["file"], file.as_str());
    assert_eq!(value["line"], 1);
    assert_eq!(value["col"], 4);
    assert_eq!(value["msg"], "not in canonical form");
    assert_eq!(value["canonical"], "fn main -> unit\n nil\n");
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
