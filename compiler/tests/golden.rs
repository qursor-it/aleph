//! Golden test harness for `tests/aph` (T-015).
//!
//! Conventions:
//!
//! - `aleph check`, `aleph fmt` and `aleph ast` run from the repository root
//!   with a repo-relative argument such as `tests/aph/NAME.aph`, so their
//!   diagnostics carry the repo-relative path.
//! - `aleph run` runs from a fresh temporary directory with just `NAME.aph`
//!   and the program arguments, so runtime diagnostics carry the basename.
//!   The temporary directory contains a copy of the source and of
//!   `tests/aph/data/` when that directory exists.
//! - `NAME.diag` is `aleph check` stdout byte for byte: empty means a clean
//!   file (exit 0), non-empty means diagnostics (exit 1).
//! - `NAME.ast` and `NAME.fmt` are the stdout of `aleph ast` and `aleph fmt`.
//! - `NAME.explain` and `NAME.explain.json` are the stdout of
//!   `aleph explain` and `aleph explain --json`.
//! - `NAME.out` is the stdout of `aleph run`. Optional companions are
//!   `NAME.args` (one argument per line), `NAME.exit` (expected exit code,
//!   default 0), `NAME.err` (expected stderr) and `NAME.files` (safe relative
//!   paths that must exist afterwards, compared with
//!   `tests/aph/expect/<path>`).
//! - `bench_maxmin.aph` and `bench_maxmin_none.aph` are intentional twins:
//!   the latter exercises the no-arguments error path (stderr `no numbers`,
//!   exit 1) while keeping the same helpers and examples.
//!
//! Failures are collected so one run reports every mismatch.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Sidecar extensions that belong to a `NAME.aph` source.
const SIDECARS: [&str; 9] = [
    "out", "diag", "ast", "fmt", "explain", "args", "exit", "err", "files",
];
/// Extensions that only make sense next to a `NAME.out` run golden.
const RUN_MODIFIERS: [&str; 4] = ["args", "exit", "err", "files"];
/// At least one of these makes a source file a golden case.
const COMPANIONS: [&str; 5] = ["out", "diag", "ast", "fmt", "explain"];
/// A `NAME.explain.json` sidecar has a two-part extension.
const EXPLAIN_JSON_SUFFIX: &str = ".explain.json";
/// A spawned `aleph` process that runs longer than this is killed.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

/// Run `aleph` in `directory`, waiting at most [`PROCESS_TIMEOUT`].
///
/// The child is polled with `try_wait`; on timeout it is killed and an error is
/// returned. The caller records that error as a test failure.
fn aleph_in<S: AsRef<OsStr>>(directory: &Path, args: &[S]) -> Result<Output, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn aleph: {error}"))?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= PROCESS_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "aleph did not finish within {} s",
                        PROCESS_TIMEOUT.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed to wait for aleph: {error}"));
            }
        }
    }

    child
        .wait_with_output()
        .map_err(|error| format!("failed to collect aleph output: {error}"))
}

fn aleph(args: &[&str]) -> Result<Output, String> {
    aleph_in(&repo_root(), args)
}

fn cases_with(extension: &str) -> Vec<PathBuf> {
    let directory = repo_root().join("tests/aph");
    let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("tests/aph exists")
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
        .filter(|path| path.with_extension(extension).is_file())
        .collect();
    cases.sort();
    assert!(
        !cases.is_empty(),
        "no .aph/.{extension} cases in {directory:?}"
    );
    cases
}

fn relative<'a>(repo: &'a Path, path: &'a Path) -> &'a str {
    path.strip_prefix(repo)
        .expect("case is inside the repo")
        .to_str()
        .expect("case path is UTF-8")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn show_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Read a golden sidecar. A missing file is empty; any other error is a test
/// failure.
fn read_lines(path: &Path) -> Result<Vec<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text.lines().map(str::to_string).collect()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// Validate a `.files` entry: only plain relative components are allowed, so
/// joining it to the temporary directory can never escape that directory.
fn checked_relative(file: &str) -> Result<PathBuf, String> {
    if file.is_empty() {
        return Err(".files entry is empty".to_string());
    }
    let path = Path::new(file);
    if path.is_absolute() {
        return Err(format!(".files entry `{file}` is absolute"));
    }
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            ".files entry `{file}` must be a plain relative path without `.`/`..`, root or prefix components"
        ));
    }
    Ok(path.to_path_buf())
}

/// A temporary directory removed by [`Drop`], including on test failure.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create a unique directory with `fs::create_dir`. An existing name
    /// (EEXIST) simply advances the counter.
    fn new() -> io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        loop {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("aleph-golden-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create copied directory");
    for entry in fs::read_dir(source).expect("read source directory") {
        let entry = entry.expect("readable directory entry");
        let target = destination.join(entry.file_name());
        let file_type = entry.file_type().expect("read file type");
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// Whether a `NAME.aph` source has at least one golden companion.
fn has_companion(path: &Path) -> bool {
    COMPANIONS
        .iter()
        .any(|companion| path.with_extension(companion).is_file())
        || path.with_extension("explain.json").is_file()
}

/// The `.aph` source a `NAME.explain.json` sidecar belongs to.
fn explain_json_source(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(EXPLAIN_JSON_SUFFIX)?;
    Some(path.with_file_name(format!("{stem}.aph")))
}

#[test]
fn golden_files_are_consistent() {
    let directory = repo_root().join("tests/aph");
    let mut paths: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("tests/aph exists")
        .map(|entry| entry.expect("readable dir entry").path())
        .collect();
    paths.sort();

    let mut orphan_sources = Vec::new();
    let mut missing_sources = Vec::new();
    let mut missing_run_goldens = Vec::new();

    for path in paths {
        let name = file_name(&path);

        if path.extension().and_then(|ext| ext.to_str()) == Some("aph") {
            if !has_companion(&path) {
                orphan_sources.push(name);
            }
            continue;
        }

        if let Some(source) = explain_json_source(&path) {
            if !source.is_file() {
                missing_sources.push(name);
            }
            continue;
        }

        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !SIDECARS.contains(&extension) {
            continue;
        }

        if !path.with_extension("aph").is_file() {
            missing_sources.push(name);
            continue;
        }
        if RUN_MODIFIERS.contains(&extension) && !path.with_extension("out").is_file() {
            missing_run_goldens.push(name);
        }
    }

    let mut problems = Vec::new();
    if !orphan_sources.is_empty() {
        problems.push(format!(
            "orphan .aph files (need one of .out/.diag/.ast/.fmt/.explain/.explain.json): {}",
            orphan_sources.join(", ")
        ));
    }
    if !missing_sources.is_empty() {
        problems.push(format!(
            "sidecar files without a matching .aph source: {}",
            missing_sources.join(", ")
        ));
    }
    if !missing_run_goldens.is_empty() {
        problems.push(format!(
            ".args/.exit/.err/.files without a matching .out: {}",
            missing_run_goldens.join(", ")
        ));
    }

    assert!(
        problems.is_empty(),
        "golden file consistency failure(s):\n{}",
        problems.join("\n")
    );
}

#[test]
fn diag_golden_files_match() {
    let repo = repo_root();
    let mut failures = Vec::new();
    for aph in cases_with("diag") {
        let relative = relative(&repo, &aph);
        let output = match aleph(&["check", relative]) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        let expected = fs::read(aph.with_extension("diag")).expect("read .diag");
        let expected_code = if expected.is_empty() { 0 } else { 1 };

        if output.status.code() != Some(expected_code) {
            failures.push(format!(
                "{relative}: expected exit {expected_code}, got {:?}",
                output.status.code()
            ));
        }
        if !output.stderr.is_empty() {
            failures.push(format!(
                "{relative}: unexpected stderr: {}",
                show_bytes(&output.stderr)
            ));
        }
        if output.stdout != expected {
            failures.push(format!(
                "{relative}: check stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&output.stdout)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} diagnostic golden failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn fmt_golden_files_match() {
    let repo = repo_root();
    let mut failures = Vec::new();
    for aph in cases_with("fmt") {
        let relative = relative(&repo, &aph);
        let output = match aleph(&["fmt", relative]) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        let expected = fs::read(aph.with_extension("fmt")).expect("read .fmt");

        if output.status.code() != Some(0) {
            failures.push(format!(
                "{relative}: fmt expected exit 0, got {:?}",
                output.status.code()
            ));
        }
        if !output.stderr.is_empty() {
            failures.push(format!(
                "{relative}: unexpected stderr: {}",
                show_bytes(&output.stderr)
            ));
        }
        if output.stdout != expected {
            failures.push(format!(
                "{relative}: fmt stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&output.stdout)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} fmt golden failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn ast_golden_files_match() {
    let repo = repo_root();
    let mut failures = Vec::new();
    for aph in cases_with("ast") {
        let relative = relative(&repo, &aph);
        let output = match aleph(&["ast", relative]) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        let expected = fs::read(aph.with_extension("ast")).expect("read .ast");

        if output.status.code() != Some(0) {
            failures.push(format!(
                "{relative}: ast expected exit 0, got {:?}",
                output.status.code()
            ));
        }
        if !output.stderr.is_empty() {
            failures.push(format!(
                "{relative}: unexpected stderr: {}",
                show_bytes(&output.stderr)
            ));
        }
        if output.stdout != expected {
            failures.push(format!(
                "{relative}: ast stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&output.stdout)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} ast golden failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn explain_golden_files_match() {
    let repo = repo_root();
    let mut failures = Vec::new();
    for aph in cases_with("explain") {
        let relative = relative(&repo, &aph);
        let output = match aleph(&["explain", relative]) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        let expected = fs::read(aph.with_extension("explain")).expect("read .explain");

        if output.status.code() != Some(0) {
            failures.push(format!(
                "{relative}: explain expected exit 0, got {:?} (stderr: {})",
                output.status.code(),
                show_bytes(&output.stderr)
            ));
        }
        if !output.stderr.is_empty() {
            failures.push(format!(
                "{relative}: unexpected stderr: {}",
                show_bytes(&output.stderr)
            ));
        }
        if output.stdout != expected {
            failures.push(format!(
                "{relative}: explain stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&output.stdout)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} explain golden failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn explain_json_golden_files_match() {
    let repo = repo_root();
    let mut failures = Vec::new();
    for aph in cases_with("explain.json") {
        let relative = relative(&repo, &aph);
        let output = match aleph(&["explain", "--json", relative]) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        let expected = fs::read(aph.with_extension("explain.json")).expect("read .explain.json");

        if output.status.code() != Some(0) {
            failures.push(format!(
                "{relative}: explain --json expected exit 0, got {:?} (stderr: {})",
                output.status.code(),
                show_bytes(&output.stderr)
            ));
        }
        if !output.stderr.is_empty() {
            failures.push(format!(
                "{relative}: unexpected stderr: {}",
                show_bytes(&output.stderr)
            ));
        }
        if output.stdout != expected {
            failures.push(format!(
                "{relative}: explain --json stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&output.stdout)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} explain JSON golden failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn run_golden_files_match() {
    let repo = repo_root();
    let golden_dir = repo.join("tests/aph");
    let mut failures = Vec::new();

    for aph in cases_with("out") {
        let relative = relative(&repo, &aph);
        let name = file_name(&aph);

        // 1. `aleph check` must accept the source silently.
        let check = match aleph(&["check", relative]) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        if check.status.code() != Some(0) {
            failures.push(format!(
                "{relative}: check expected exit 0, got {:?} (stdout: {})",
                check.status.code(),
                show_bytes(&check.stdout)
            ));
        }
        if !check.stdout.is_empty() {
            failures.push(format!(
                "{relative}: check produced diagnostics: {}",
                show_bytes(&check.stdout)
            ));
        }
        if !check.stderr.is_empty() {
            failures.push(format!(
                "{relative}: check wrote stderr: {}",
                show_bytes(&check.stderr)
            ));
        }

        // 2. Read and validate the `.files` entries before touching the temp
        //    directory or the expected-file tree.
        let expected_lines = match read_lines(&aph.with_extension("files")) {
            Ok(lines) => lines,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };
        let mut expected_files = Vec::new();
        let mut invalid_files = false;
        for line in &expected_lines {
            match checked_relative(line) {
                Ok(file) => expected_files.push(file),
                Err(error) => {
                    failures.push(format!("{relative}: {error}"));
                    invalid_files = true;
                }
            }
        }
        if invalid_files {
            continue;
        }

        let program_args = match read_lines(&aph.with_extension("args")) {
            Ok(lines) => lines,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };

        // 3. Build the hermetic run directory.
        let temp = match TempDir::new() {
            Ok(temp) => temp,
            Err(error) => {
                failures.push(format!(
                    "{relative}: cannot create temporary directory: {error}"
                ));
                continue;
            }
        };
        fs::copy(&aph, temp.path().join(&name)).expect("copy case source");
        let data = golden_dir.join("data");
        if data.is_dir() {
            copy_dir(&data, &temp.path().join("data"));
        }
        for file in &expected_files {
            if let Some(parent) = file.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(temp.path().join(parent)).expect("create output directory");
                }
            }
        }

        // 4. Run with the optional program arguments.
        let mut command: Vec<&str> = vec!["run", name.as_str()];
        command.extend(program_args.iter().map(String::as_str));
        let output = match aleph_in(temp.path(), &command) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{relative}: {error}"));
                continue;
            }
        };

        let expected_code = match fs::read_to_string(aph.with_extension("exit")) {
            Ok(text) => text
                .trim()
                .parse::<i32>()
                .expect("NAME.exit contains an integer"),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(error) => {
                failures.push(format!(
                    "{relative}: cannot read {}: {error}",
                    aph.with_extension("exit").display()
                ));
                continue;
            }
        };
        if output.status.code() != Some(expected_code) {
            failures.push(format!(
                "{relative}: expected exit {expected_code}, got {:?} (stderr: {})",
                output.status.code(),
                show_bytes(&output.stderr)
            ));
        }

        let expected_stdout = fs::read(aph.with_extension("out")).expect("read .out");
        if output.stdout != expected_stdout {
            failures.push(format!(
                "{relative}: stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected_stdout),
                show_bytes(&output.stdout)
            ));
        }

        let err_path = aph.with_extension("err");
        if err_path.is_file() {
            let expected_stderr = fs::read(&err_path).expect("read .err");
            if output.stderr != expected_stderr {
                failures.push(format!(
                    "{relative}: stderr mismatch\nexpected:\n{}\nactual:\n{}",
                    show_bytes(&expected_stderr),
                    show_bytes(&output.stderr)
                ));
            }
        } else if !output.stderr.is_empty() {
            failures.push(format!(
                "{relative}: unexpected stderr: {}",
                show_bytes(&output.stderr)
            ));
        }

        // 5. Check files the program was expected to write.
        for file in &expected_files {
            let expected_path = golden_dir.join("expect").join(file);
            let expected = match fs::read(&expected_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    failures.push(format!(
                        "{relative}: cannot read expected file {}: {error}",
                        expected_path.display()
                    ));
                    continue;
                }
            };
            match fs::read(temp.path().join(file)) {
                Ok(actual) if actual == expected => {}
                Ok(actual) => failures.push(format!(
                    "{relative}: file {} mismatch\nexpected:\n{}\nactual:\n{}",
                    file.display(),
                    show_bytes(&expected),
                    show_bytes(&actual)
                )),
                Err(error) => failures.push(format!(
                    "{relative}: expected file {} was not written: {error}",
                    file.display()
                )),
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} run golden failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every run golden is canonical (`aleph fmt` is a no-op) and `aleph check`
/// is silent.
#[test]
fn run_goldens_are_canonical_and_check_clean() {
    let repo = repo_root();
    let mut failures = Vec::new();
    for aph in cases_with("out") {
        let relative = relative(&repo, &aph);

        match aleph(&["fmt", relative]) {
            Ok(fmt) => {
                let source = fs::read(&aph).expect("read .aph");
                if fmt.status.code() != Some(0) {
                    failures.push(format!(
                        "{relative}: not parseable (stderr: {})",
                        show_bytes(&fmt.stderr)
                    ));
                } else if fmt.stdout != source {
                    failures.push(format!("{relative}: not canonical"));
                }
            }
            Err(error) => failures.push(format!("{relative}: {error}")),
        }

        match aleph(&["check", relative]) {
            Ok(check) => {
                if check.status.code() != Some(0) {
                    failures.push(format!(
                        "{relative}: check exit {:?} (stdout: {})",
                        check.status.code(),
                        show_bytes(&check.stdout)
                    ));
                }
                if !check.stdout.is_empty() {
                    failures.push(format!(
                        "{relative}: check produced diagnostics: {}",
                        show_bytes(&check.stdout)
                    ));
                }
                if !check.stderr.is_empty() {
                    failures.push(format!(
                        "{relative}: check wrote stderr: {}",
                        show_bytes(&check.stderr)
                    ));
                }
            }
            Err(error) => failures.push(format!("{relative}: {error}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} canonical/check failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
