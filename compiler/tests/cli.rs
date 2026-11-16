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

/// The compiler to probe: `$CC` when set, otherwise `cc`.
///
/// `CC` must be a bare executable path, not a command line with arguments.
fn cc_program() -> std::ffi::OsString {
    std::env::var_os("CC")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cc".into())
}

fn cc_is_missing() -> bool {
    match Command::new(cc_program()).arg("--version").output() {
        Ok(_) => false,
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

fn skip_without_cc(label: &str) -> bool {
    if !cc_is_missing() {
        return false;
    }
    if std::env::var("ALEPH_REQUIRE_CC").ok().as_deref() == Some("1") {
        panic!("cc is required by ALEPH_REQUIRE_CC=1 ({label})");
    }
    println!("build: cc is not available; skipping {label}");
    true
}

fn temp_output(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("aleph-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

fn run_built(path: &Path, args: &[&str]) -> Output {
    Command::new(path)
        .args(args)
        .output()
        .expect("failed to run the built program")
}

fn build_program(source: &str, name: &str) -> PathBuf {
    let binary = temp_output(name);
    let out = run(&["build", source, "-o", binary.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "build stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "build stdout must be empty");
    assert!(out.stderr.is_empty(), "build stderr must be empty");
    binary
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

/// `tokens` was the last stubbed subcommand (X001); it now reads its file.
#[test]
fn tokens_missing_file_reports_x002() {
    let file = "foo.aph";
    let out = run(&["tokens", file]);
    assert_eq!(out.status.code(), Some(1));

    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X002");
    assert_eq!(value["file"], file);
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

const FIB_SOURCE: &str = "fn fib n:i64 -> i64\n doc \"nth fibonacci number\"\n ex fib 0 = 0\n ex fib 10 = 55\n if < n 2 n + fib - n 1 fib - n 2\nfn main -> unit\n print show fib 20\n";

#[test]
fn build_compiles_spec12_fib_and_prints_6765() {
    if skip_without_cc("SPEC §12 fib") {
        return;
    }
    let source = temp_source("build_fib.aph", FIB_SOURCE);
    let binary = temp_output("build_fib");
    let built = run(&["build", source.as_str(), "-o", binary.to_str().unwrap()]);
    assert_eq!(
        built.status.code(),
        Some(0),
        "build stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(built.stdout.is_empty(), "build stdout must be empty");
    assert!(built.stderr.is_empty(), "build stderr must be empty");

    let executed = run_built(&binary, &[]);
    assert_eq!(executed.status.code(), Some(0));
    assert_eq!(String::from_utf8(executed.stdout).unwrap(), "6765\n");
    assert!(executed.stderr.is_empty());
}

#[test]
fn build_records_opt_res_try_matches_the_interpreter() {
    if skip_without_cc("records/opt/res/try") {
        return;
    }
    let source = temp_source(
        "build_records.aph",
        "type point x:i64 y:i64\nfn shift p:point d:i64 -> point\n point + .x p d .y p\nfn pick o:opt i64 -> i64\n else o 0\nfn plus r:res i64 str -> res i64 str\n let n try r\n ok + n 1\nfn main -> res unit str\n let p shift point 1 2 10\n do print show .x p\n do print show .y p\n do print show pick some 5\n do print show pick none i64\n do print show else ok 7 0\n do print show else err \"x\" 9\n do print show has some 1\n do print show isok plus ok 41\n do print show isok plus err \"x\"\n let n try plus ok 41\n do print show n\n ok nil\n",
    );
    let binary = temp_output("build_records");
    let built = run(&["build", source.as_str(), "-o", binary.to_str().unwrap()]);
    assert_eq!(
        built.status.code(),
        Some(0),
        "build stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(native.stderr, interpreted.stderr);
    assert_eq!(
        String::from_utf8(native.stdout).unwrap(),
        "11\n2\n5\n0\n7\n9\ntrue\ntrue\nfalse\n42\n"
    );
}

#[test]
fn build_trap_json_matches_the_interpreter() {
    if skip_without_cc("R001 trap") {
        return;
    }
    let source = temp_source(
        "build_trap.aph",
        "fn main -> unit\n do print \"before\"\n print show + 9223372036854775807 1\n",
    );
    let binary = temp_output("build_trap");
    let built = run(&["build", source.as_str(), "-o", binary.to_str().unwrap()]);
    assert_eq!(
        built.status.code(),
        Some(0),
        "build stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), Some(70));
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(native.stderr, interpreted.stderr);

    let value: serde_json::Value =
        serde_json::from_slice(&native.stderr).expect("stderr must be valid JSON");
    assert_eq!(value["code"], "R001");
    assert_eq!(value["file"], source.as_str());
}

#[test]
fn build_main_returning_err_exits_one() {
    if skip_without_cc("main err") {
        return;
    }
    let source = temp_source(
        "build_main_err.aph",
        "fn main -> res unit str\n err \"bad\"\n",
    );
    let binary = temp_output("build_main_err");
    let built = run(&["build", source.as_str(), "-o", binary.to_str().unwrap()]);
    assert_eq!(built.status.code(), Some(0));

    let native = run_built(&binary, &[]);
    assert_eq!(native.status.code(), Some(1));
    assert!(native.stdout.is_empty());
    assert_eq!(String::from_utf8(native.stderr).unwrap(), "bad\n");
}

#[test]
fn build_passes_program_arguments() {
    if skip_without_cc("program arguments") {
        return;
    }
    let source = temp_source("build_args.aph", "fn main -> unit\n print show args\n");
    let binary = temp_output("build_args");
    let built = run(&["build", source.as_str(), "-o", binary.to_str().unwrap()]);
    assert_eq!(built.status.code(), Some(0));

    let native = run_built(&binary, &["--json", "-w", "two words"]);
    let interpreted = run(&["run", source.as_str(), "--json", "-w", "two words"]);
    assert_eq!(native.status.code(), Some(0));
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(
        String::from_utf8(native.stdout).unwrap(),
        "[\"--json\" \"-w\" \"two words\"]\n"
    );
}

#[test]
fn build_without_output_is_a_usage_error() {
    let out = run(&["build", "foo.aph"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn build_map_filter_fold_with_user_functions_matches_the_interpreter() {
    if skip_without_cc("map/filter/fold user functions") {
        return;
    }
    let source = temp_source(
        "build_hof_user.aph",
        "fn inc n:i64 -> i64\n + n 1\nfn even n:i64 -> bool\n == 0 % n 2\nfn add a:i64 b:i64 -> i64\n + a b\nfn main -> unit\n do print show map @inc [1 2 3]\n do print show filter @even [1 2 3 4]\n do print show fold @add 0 [1 2 3 4]\n nil\n",
    );
    let binary = build_program(&source, "build_hof_user");
    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(native.stderr, interpreted.stderr);
    assert_eq!(
        String::from_utf8(native.stdout).unwrap(),
        "[2 3 4]\n[2 4]\n10\n"
    );
}

#[test]
fn build_map_with_constructor_and_builtin_references_matches_the_interpreter() {
    if skip_without_cc("map with constructor/builtin references") {
        return;
    }
    let source = temp_source(
        "build_hof_refs.aph",
        "type box v:i64\nfn main -> unit\n do print show len map @box [1 2 3]\n do print show map @show [1 2 3]\n do print show map @len [empty i64 empty i64]\n nil\n",
    );
    let binary = build_program(&source, "build_hof_refs");
    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(native.stderr, interpreted.stderr);
    assert_eq!(
        String::from_utf8(native.stdout).unwrap(),
        "3\n[\"1\" \"2\" \"3\"]\n[0 0]\n"
    );
}

#[test]
fn build_fold_with_res_accumulator_matches_the_interpreter() {
    if skip_without_cc("fold with res accumulator") {
        return;
    }
    let source = temp_source(
        "build_fold_res.aph",
        "fn addres acc:res i64 str n:i64 -> res i64 str\n let a try acc\n ok + a n\nfn total xs:list i64 -> res i64 str\n fold @addres ok 0 xs\nfn main -> res unit str\n let n try total [1 2 3]\n do print show n\n do print show isok fold @addres err \"x\" [1 2]\n ok nil\n",
    );
    let binary = build_program(&source, "build_fold_res");
    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(native.stderr, interpreted.stderr);
    assert_eq!(String::from_utf8(native.stdout).unwrap(), "6\nfalse\n");
}

#[test]
fn build_trap_inside_mapped_function_matches_the_interpreter() {
    if skip_without_cc("trap inside map") {
        return;
    }
    let source = temp_source(
        "build_map_trap.aph",
        "fn div n:i64 -> i64\n / 10 n\nfn main -> unit\n print show map @div [2 0 5]\n",
    );
    let binary = build_program(&source, "build_map_trap");
    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), Some(70));
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(native.stderr, interpreted.stderr);

    let value: serde_json::Value =
        serde_json::from_slice(&native.stderr).expect("stderr must be valid JSON");
    assert_eq!(value["code"], "R002");
    assert_eq!(value["file"], source.as_str());
}

#[test]
fn build_handles_user_bindings_named_after_generated_identifiers() {
    if skip_without_cc("reserved generated names") {
        return;
    }
    let source = temp_source(
        "build_reserved_names.aph",
        "fn f result:i64 -> i64\n let value result\n let thing value\n let n0 thing\n + n0 1\nfn main -> unit\n let result f 1\n do print show result\n nil\n",
    );
    let binary = build_program(&source, "build_reserved_names");
    let native = run_built(&binary, &[]);
    let interpreted = run(&["run", source.as_str()]);
    assert_eq!(native.status.code(), Some(0));
    assert_eq!(native.stdout, interpreted.stdout);
    assert_eq!(String::from_utf8(native.stdout).unwrap(), "2\n");
}

#[cfg(unix)]
#[test]
fn build_reports_x004_when_the_c_compiler_fails() {
    let source = temp_source("build_cc_fail.aph", "fn main -> unit\n nil\n");
    let binary = temp_output("build_cc_fail");
    let out = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .env("CC", "/bin/false")
        .args(["build", source.as_str(), "-o", binary.to_str().unwrap()])
        .output()
        .expect("failed to run aleph");

    assert_eq!(out.status.code(), Some(1));
    assert!(!binary.exists());
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X004");
    assert_eq!(value["msg"], "c compiler failed");
    assert_eq!(value["file"], source.as_str());
}

#[test]
fn build_without_main_reports_x003() {
    let source = temp_source("build_no_main.aph", "fn helper -> i64\n 1\n");
    let binary = temp_output("build_no_main");
    let out = run(&["build", source.as_str(), "-o", binary.to_str().unwrap()]);

    assert_eq!(out.status.code(), Some(1));
    assert!(!binary.exists());
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["code"], "X003");
    assert_eq!(value["msg"], "no main function");
}

#[test]
fn build_deep_recursion_with_a_large_list_literal_does_not_segv() {
    if skip_without_cc("deep recursion with a large list") {
        return;
    }
    let items = (1..=3000)
        .map(|number| number.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let source = temp_source(
        "build_deep_list.aph",
        &format!(
            "fn walk n:i64 -> i64\n let xs [{items}]\n if == n 0 len xs + len xs walk - n 1\nfn main -> unit\n print show walk 5000\n"
        ),
    );
    let interpreted = run(&["run", source.as_str()]);
    let binary = build_program(&source, "build_deep_list");
    let native = run_built(&binary, &[]);

    match native.status.code() {
        Some(0) => {
            assert_eq!(
                interpreted.status.code(),
                Some(0),
                "native succeeded but the interpreter did not: {}",
                String::from_utf8_lossy(&interpreted.stderr)
            );
            assert_eq!(native.stdout, interpreted.stdout);
        }
        Some(70) => {
            let value: serde_json::Value =
                serde_json::from_slice(&native.stderr).expect("stderr must be valid JSON");
            assert_eq!(value["code"], "R004");
        }
        other => panic!(
            "native terminated abnormally: {other:?}; stderr: {}",
            String::from_utf8_lossy(&native.stderr)
        ),
    }
}

#[test]
fn edit_prints_the_edited_text_and_leaves_the_file() {
    let source = "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 1\n";
    let file = temp_source("edit_print.aph", source);
    let edits = r#"[{"op":"replace","node":"n9","text":"+ x 2"},{"op":"replace","node":"n5","text":"ex inc 1 = 3"}]"#;
    let out = run(&["edit", file.as_str(), "--json", edits]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "fn inc x:i64 -> i64\n ex inc 1 = 3\n + x 2\n"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
}

#[test]
fn edit_write_applies_and_reports_check_diagnostics() {
    let file = temp_source(
        "edit_write.aph",
        "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 1\n",
    );
    let out = run(&[
        "edit",
        file.as_str(),
        "-w",
        "--json",
        r#"[{"op":"replace","node":"n9","text":"+ x 2"}]"#,
    ]);
    // The edit is written; the stale example then fails with E002.
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"code\":\"E002\""));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 2\n"
    );
}

#[test]
fn edit_rejects_unparsable_results_without_writing() {
    let source = "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 1\n";
    let file = temp_source("edit_reject.aph", source);
    let out = run(&[
        "edit",
        file.as_str(),
        "-w",
        "--json",
        r#"[{"op":"replace","node":"n9","text":"+ x"}]"#,
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"code\":\"X010\""));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
}

#[test]
fn tokens_counts_with_the_default_and_all_tokenizers() {
    let file = temp_source("tokens.aph", "fn main -> unit\n print \"hi\"\n");
    let out = run(&["tokens", file.as_str()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "chars4 7\n");

    let out = run(&["tokens", file.as_str(), "--tokenizer", "all", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let names: Vec<&str> = report["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["tokenizer"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["chars4", "bytes", "chars", "words", "aleph"]);
    assert_eq!(report["tokens"][4]["count"], 6);
}

#[test]
fn tokens_rejects_unknown_tokenizers_and_lex_errors() {
    let file = temp_source("tokens_bad.aph", "fn main -> unit\n print $\n");
    let out = run(&["tokens", file.as_str(), "--tokenizer", "gpt4"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown tokenizer"));

    let out = run(&["tokens", file.as_str(), "--tokenizer", "aleph"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"code\":\"L"));
}
