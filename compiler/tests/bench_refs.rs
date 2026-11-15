//! Reference implementations for the benchmark tasks (T-021).
//!
//! `bench/ref/c/NAME.c` and `bench/ref/py/NAME.py` must reproduce exactly the
//! Aleph golden stdout/stderr/exit and written files of `tests/aph/bench_NAME`.
//! Each reference runs once in a fresh hermetic directory with `data/`, `out/`
//! and the task `.args`, mirroring `bench/run.sh`.
//!
//! The C test skips when `cc` is missing unless `ALEPH_REQUIRE_CC=1`; the
//! Python test always skips with a notice when `python3` is missing.

mod common;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use common::{
    checked_relative, prepare_case, read_expected, read_lines, repo_root, show_bytes, TempDir,
};

const TASKS: [&str; 16] = [
    "fib",
    "total",
    "evens",
    "primes",
    "fizzbuzz",
    "palindrome",
    "wordcount",
    "linecount",
    "maxmin",
    "points",
    "reverse",
    "gcd",
    "upper",
    "histogram",
    "collatz",
    "fib35",
];

/// The golden sidecar prefix for a task (`fib35` lives under `bench/heavy`).
fn golden_prefix(task: &str) -> PathBuf {
    if task == "fib35" {
        repo_root().join("bench/heavy/fib35")
    } else {
        repo_root().join(format!("tests/aph/bench_{task}"))
    }
}

/// The compiler to probe: `$CC` when set, otherwise `cc`.
fn cc_program() -> OsString {
    env::var_os("CC")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cc".into())
}

fn cc_is_missing() -> bool {
    match Command::new(cc_program()).arg("--version").output() {
        Ok(_) => false,
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

/// Return true when the C test should skip because `cc` is absent.
fn skip_without_cc(label: &str) -> bool {
    if !cc_is_missing() {
        return false;
    }
    if let Some(value) = env::var_os("CC").filter(|value| !value.is_empty()) {
        panic!("CC is set to {value:?} but it is not runnable ({label})");
    }
    if env::var("ALEPH_REQUIRE_CC").ok().as_deref() == Some("1") {
        panic!("cc is required by ALEPH_REQUIRE_CC=1 ({label})");
    }
    println!("bench_refs: cc is not available; skipping {label}");
    true
}

fn python_is_missing() -> bool {
    match Command::new("python3").arg("--version").output() {
        Ok(_) => false,
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

/// A prepared hermetic case plus the expected outcomes of its golden.
struct Case {
    task: &'static str,
    temp: TempDir,
    expected_stdout: Vec<u8>,
    expected_stderr: Vec<u8>,
    expected_exit: i32,
    args: Vec<String>,
    expected_files: Vec<PathBuf>,
}

fn expected_exit(aph: &std::path::Path) -> Result<i32, String> {
    match fs::read_to_string(aph.with_extension("exit")) {
        Ok(text) => text
            .trim()
            .parse::<i32>()
            .map_err(|error| format!("NAME.exit is not an integer: {error}")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(format!(
            "cannot read {}: {error}",
            aph.with_extension("exit").display()
        )),
    }
}

fn load_case(task: &'static str) -> Result<Case, String> {
    let aph = golden_prefix(task).with_extension("aph");
    let expected_files = read_lines(&aph.with_extension("files"))?
        .iter()
        .map(|line| checked_relative(line))
        .collect::<Result<Vec<_>, _>>()?;
    let args = read_lines(&aph.with_extension("args"))?;
    let expected_stdout = read_expected(&aph.with_extension("out"))?;
    let expected_stderr = read_expected(&aph.with_extension("err"))?;
    let expected_exit = expected_exit(&aph)?;
    let temp = TempDir::new()
        .map_err(|error| format!("{task}: cannot create temporary directory: {error}"))?;
    prepare_case(&temp, &aph, &expected_files).map_err(|error| format!("{task}: {error}"))?;
    Ok(Case {
        task,
        temp,
        expected_stdout,
        expected_stderr,
        expected_exit,
        args,
        expected_files,
    })
}

fn clear_files(case: &Case) {
    for file in &case.expected_files {
        let _ = fs::remove_file(case.temp.path().join(file));
    }
}

/// Compare one reference run with the golden and record every mismatch.
fn check_output(case: &Case, engine: &str, output: &common::Output, failures: &mut Vec<String>) {
    let name = format!("bench_{}", case.task);

    if output.status.code() != Some(case.expected_exit) {
        failures.push(format!(
            "{name} ({engine}): exit {:?}, expected {} (stderr: {})",
            output.status.code(),
            case.expected_exit,
            show_bytes(&output.stderr)
        ));
    }
    if output.stdout != case.expected_stdout {
        failures.push(format!(
            "{name} ({engine}): stdout mismatch\nexpected:\n{}\nactual:\n{}",
            show_bytes(&case.expected_stdout),
            show_bytes(&output.stdout)
        ));
    }
    if output.stderr != case.expected_stderr {
        failures.push(format!(
            "{name} ({engine}): stderr mismatch\nexpected:\n{}\nactual:\n{}",
            show_bytes(&case.expected_stderr),
            show_bytes(&output.stderr)
        ));
    }

    let expect_dir = repo_root().join("tests/aph/expect");
    for file in &case.expected_files {
        let expected = match fs::read(expect_dir.join(file)) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!(
                    "{name} ({engine}): cannot read expected file {}: {error}",
                    expect_dir.join(file).display()
                ));
                continue;
            }
        };
        match fs::read(case.temp.path().join(file)) {
            Ok(actual) if actual == expected => {}
            Ok(actual) => failures.push(format!(
                "{name} ({engine}): file {} mismatch\nexpected:\n{}\nactual:\n{}",
                file.display(),
                show_bytes(&expected),
                show_bytes(&actual)
            )),
            Err(error) => failures.push(format!(
                "{name} ({engine}): expected file {} was not written: {error}",
                file.display()
            )),
        }
    }
}

#[test]
fn c_refs_match_aleph_goldens() {
    if skip_without_cc("bench/ref/c") {
        return;
    }

    let mut failures = Vec::new();
    for task in TASKS {
        let case = match load_case(task) {
            Ok(case) => case,
            Err(error) => {
                failures.push(error);
                continue;
            }
        };
        let source = repo_root().join(format!("bench/ref/c/{task}.c"));
        let binary = case.temp.path().join("prog-c");
        let compiled = match Command::new(cc_program())
            .args(["-std=c99", "-O2", "-o"])
            .arg(&binary)
            .arg(&source)
            .arg("-lm")
            .output()
        {
            Ok(output) if output.status.success() => output,
            Ok(output) => {
                failures.push(format!(
                    "bench_{task} (c): compile failed\nstdout:\n{}\nstderr:\n{}",
                    show_bytes(&output.stdout),
                    show_bytes(&output.stderr)
                ));
                continue;
            }
            Err(error) => {
                failures.push(format!("bench_{task} (c): cannot run cc: {error}"));
                continue;
            }
        };
        if !compiled.stdout.is_empty() || !compiled.stderr.is_empty() {
            failures.push(format!(
                "bench_{task} (c): compiler wrote output\nstdout:\n{}\nstderr:\n{}",
                show_bytes(&compiled.stdout),
                show_bytes(&compiled.stderr)
            ));
        }

        clear_files(&case);
        let args: Vec<&str> = case.args.iter().map(String::as_str).collect();
        match common::run_separate(case.temp.path(), &binary, &args) {
            Ok(output) => check_output(&case, "c", &output, &mut failures),
            Err(error) => failures.push(format!("bench_{task} (c): {error}")),
        }
    }

    assert!(
        failures.is_empty(),
        "{} C reference failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn python_refs_match_aleph_goldens() {
    if python_is_missing() {
        println!("bench_refs: python3 is not available; skipping bench/ref/py");
        return;
    }

    let mut failures = Vec::new();
    for task in TASKS {
        let case = match load_case(task) {
            Ok(case) => case,
            Err(error) => {
                failures.push(error);
                continue;
            }
        };
        let script = repo_root().join(format!("bench/ref/py/{task}.py"));
        let mut args: Vec<OsString> = vec![script.into_os_string()];
        args.extend(case.args.iter().map(OsString::from));

        clear_files(&case);
        match common::run_separate(case.temp.path(), "python3", &args) {
            Ok(output) => check_output(&case, "python", &output, &mut failures),
            Err(error) => failures.push(format!("bench_{task} (python): {error}")),
        }
    }

    assert!(
        failures.is_empty(),
        "{} Python reference failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
