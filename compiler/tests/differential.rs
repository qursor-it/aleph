//! Differential tests: interpreter vs native backend (T-020, ADR-0006).
//!
//! Every `tests/aph/NAME.aph` with a `NAME.out` golden is a case. Each case
//! runs twice in a fresh hermetic temporary directory:
//!
//! - the reference interpreter via `aleph run NAME.aph ARGS`;
//! - the C backend via `aleph build NAME.aph -o prog` followed by
//!   `./prog ARGS`.
//!
//! Both runs start from a copy of the source, `tests/aph/data/` and
//! `NAME.args`, in independent directories so written files cannot interfere.
//! The test asserts that stdout, stderr and the exit code are byte-identical
//! between the two engines, that both match `NAME.out`/`NAME.err`/`NAME.exit`,
//! and that every `NAME.files` path holds the same bytes in both directories
//! and in `tests/aph/expect/`. Cases with a `NAME.merged` sidecar are run a
//! second time with stdout and stderr redirected to one file (one `File`
//! cloned for both `Stdio` handles), so the combined stream must match too.
//!
//! Builds happen sequentially (one `cc` at a time) and failures are collected
//! so one run reports every mismatch. Like the other C-backend tests this
//! skips without `cc`, unless `ALEPH_REQUIRE_CC=1` is set. Builds add
//! `-Wall -Wextra -Werror` through `ALEPH_CFLAGS` so warnings fail (T-051).

mod common;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{
    checked_relative, file_name, prepare_case, read_expected, read_lines, repo_root, show_bytes,
    TempDir,
};

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

/// Return true when the caller should skip because `cc` is absent.
fn skip_without_cc(label: &str) -> bool {
    if !cc_is_missing() {
        return false;
    }
    if std::env::var("ALEPH_REQUIRE_CC").ok().as_deref() == Some("1") {
        panic!("cc is required by ALEPH_REQUIRE_CC=1 ({label})");
    }
    println!("differential: cc is not available; skipping {label}");
    true
}

/// Every `tests/aph/NAME.aph` that has a `NAME.out`, sorted.
fn run_cases() -> Vec<PathBuf> {
    let directory = repo_root().join("tests/aph");
    let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("tests/aph exists")
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
        .filter(|path| path.with_extension("out").is_file())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no .aph/.out cases in {directory:?}");
    cases
}

fn expected_exit(aph: &Path) -> Result<i32, String> {
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

/// Compare one case and append human-readable problems to `failures`.
fn check_case(aph: &Path, failures: &mut Vec<String>) {
    let name = file_name(aph);

    let expected_files = match read_lines(&aph.with_extension("files")) {
        Ok(lines) => {
            let mut files = Vec::new();
            let mut invalid = false;
            for line in &lines {
                match checked_relative(line) {
                    Ok(file) => files.push(file),
                    Err(error) => {
                        failures.push(format!("{name}: {error}"));
                        invalid = true;
                    }
                }
            }
            if invalid {
                return;
            }
            files
        }
        Err(error) => {
            failures.push(format!("{name}: {error}"));
            return;
        }
    };
    let program_args = match read_lines(&aph.with_extension("args")) {
        Ok(lines) => lines,
        Err(error) => {
            failures.push(format!("{name}: {error}"));
            return;
        }
    };
    let exit = match expected_exit(aph) {
        Ok(exit) => exit,
        Err(error) => {
            failures.push(format!("{name}: {error}"));
            return;
        }
    };
    let expected_stdout = match read_expected(&aph.with_extension("out")) {
        Ok(bytes) => bytes,
        Err(error) => {
            failures.push(format!("{name}: {error}"));
            return;
        }
    };
    let expected_stderr = match read_expected(&aph.with_extension("err")) {
        Ok(bytes) => bytes,
        Err(error) => {
            failures.push(format!("{name}: {error}"));
            return;
        }
    };

    let interp = match TempDir::new() {
        Ok(temp) => temp,
        Err(error) => {
            failures.push(format!(
                "{name}: cannot create temporary directory: {error}"
            ));
            return;
        }
    };
    let native = match TempDir::new() {
        Ok(temp) => temp,
        Err(error) => {
            failures.push(format!(
                "{name}: cannot create temporary directory: {error}"
            ));
            return;
        }
    };
    if let Err(error) = prepare_case(&interp, aph, &expected_files) {
        failures.push(format!("{name}: {error}"));
        return;
    }
    if let Err(error) = prepare_case(&native, aph, &expected_files) {
        failures.push(format!("{name}: {error}"));
        return;
    }

    // 1. Reference interpreter.
    let mut command: Vec<&str> = vec!["run", name.as_str()];
    command.extend(program_args.iter().map(String::as_str));
    let interpreted =
        match common::run_separate(interp.path(), env!("CARGO_BIN_EXE_aleph"), &command) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{name}: interpreter: {error}"));
                return;
            }
        };

    // 2. Native build (sequential) and run.
    let built = match common::run_separate(
        native.path(),
        env!("CARGO_BIN_EXE_aleph"),
        &["build", name.as_str(), "-o", "prog"],
    ) {
        Ok(output) => output,
        Err(error) => {
            failures.push(format!("{name}: build: {error}"));
            return;
        }
    };
    if built.status.code() != Some(0) {
        failures.push(format!(
            "{name}: build failed with exit {:?}\nstdout:\n{}\nstderr:\n{}",
            built.status.code(),
            show_bytes(&built.stdout),
            show_bytes(&built.stderr)
        ));
        return;
    }
    if !built.stdout.is_empty() {
        failures.push(format!(
            "{name}: build wrote stdout: {}",
            show_bytes(&built.stdout)
        ));
    }
    if !built.stderr.is_empty() {
        failures.push(format!(
            "{name}: build wrote stderr: {}",
            show_bytes(&built.stderr)
        ));
    }

    let native_args: Vec<&str> = program_args.iter().map(String::as_str).collect();
    let native_binary = native.path().join("prog");
    let native_output = match common::run_separate(native.path(), &native_binary, &native_args) {
        Ok(output) => output,
        Err(error) => {
            failures.push(format!("{name}: native: {error}"));
            return;
        }
    };

    // 3. Interpreter vs native.
    if interpreted.status.code() != native_output.status.code() {
        failures.push(format!(
            "{name}: exit mismatch: interpreter {:?}, native {:?}",
            interpreted.status.code(),
            native_output.status.code()
        ));
    }
    if interpreted.stdout != native_output.stdout {
        failures.push(format!(
            "{name}: interpreter/native stdout mismatch\ninterpreter:\n{}\nnative:\n{}",
            show_bytes(&interpreted.stdout),
            show_bytes(&native_output.stdout)
        ));
    }
    if interpreted.stderr != native_output.stderr {
        failures.push(format!(
            "{name}: interpreter/native stderr mismatch\ninterpreter:\n{}\nnative:\n{}",
            show_bytes(&interpreted.stderr),
            show_bytes(&native_output.stderr)
        ));
    }

    // 4. Both engines vs the goldens.
    for (engine, output) in [("interpreter", &interpreted), ("native", &native_output)] {
        if output.status.code() != Some(exit) {
            failures.push(format!(
                "{name}: {engine} exit {:?}, expected {exit}",
                output.status.code()
            ));
        }
        if output.stdout != expected_stdout {
            failures.push(format!(
                "{name}: {engine} stdout mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected_stdout),
                show_bytes(&output.stdout)
            ));
        }
        if output.stderr != expected_stderr {
            failures.push(format!(
                "{name}: {engine} stderr mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected_stderr),
                show_bytes(&output.stderr)
            ));
        }
    }

    // 5. Written files: the two engines must agree and match `expect/`.
    let expect_dir = repo_root().join("tests/aph/expect");
    for file in &expected_files {
        let expected = match fs::read(expect_dir.join(file)) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!(
                    "{name}: cannot read expected file {}: {error}",
                    expect_dir.join(file).display()
                ));
                continue;
            }
        };
        let interpreted_file = match fs::read(interp.path().join(file)) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                failures.push(format!(
                    "{name}: interpreter did not write {}: {error}",
                    file.display()
                ));
                None
            }
        };
        let native_file = match fs::read(native.path().join(file)) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                failures.push(format!(
                    "{name}: native did not write {}: {error}",
                    file.display()
                ));
                None
            }
        };
        if let (Some(interpreted_file), Some(native_file)) = (interpreted_file, native_file) {
            if interpreted_file != native_file {
                failures.push(format!(
                    "{name}: written file {} differs between engines\ninterpreter:\n{}\nnative:\n{}",
                    file.display(),
                    show_bytes(&interpreted_file),
                    show_bytes(&native_file)
                ));
            }
            if interpreted_file != expected {
                failures.push(format!(
                    "{name}: interpreter file {} mismatch\nexpected:\n{}\nactual:\n{}",
                    file.display(),
                    show_bytes(&expected),
                    show_bytes(&interpreted_file)
                ));
            }
            if native_file != expected {
                failures.push(format!(
                    "{name}: native file {} mismatch\nexpected:\n{}\nactual:\n{}",
                    file.display(),
                    show_bytes(&expected),
                    show_bytes(&native_file)
                ));
            }
        }
    }

    // 6. Merged stdout+stderr ordering, when a `.merged` sidecar exists.
    let merged_golden = aph.with_extension("merged");
    if merged_golden.is_file() {
        let expected = match read_expected(&merged_golden) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                return;
            }
        };
        let interp_merged = match TempDir::new() {
            Ok(temp) => temp,
            Err(error) => {
                failures.push(format!(
                    "{name}: cannot create temporary directory: {error}"
                ));
                return;
            }
        };
        let native_merged = match TempDir::new() {
            Ok(temp) => temp,
            Err(error) => {
                failures.push(format!(
                    "{name}: cannot create temporary directory: {error}"
                ));
                return;
            }
        };
        if let Err(error) = prepare_case(&interp_merged, aph, &expected_files) {
            failures.push(format!("{name}: {error}"));
            return;
        }
        if let Err(error) = prepare_case(&native_merged, aph, &expected_files) {
            failures.push(format!("{name}: {error}"));
            return;
        }

        let interp_stream = interp_merged.path().join("_merged_stream");
        let interp_output = match common::run_merged(
            interp_merged.path(),
            env!("CARGO_BIN_EXE_aleph"),
            &command,
            &interp_stream,
        ) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{name}: interpreter merged: {error}"));
                return;
            }
        };
        let native_stream = native_merged.path().join("_merged_stream");
        let native_output = match common::run_merged(
            native_merged.path(),
            &native_binary,
            &native_args,
            &native_stream,
        ) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{name}: native merged: {error}"));
                return;
            }
        };

        if interp_output.status.code() != native_output.status.code() {
            failures.push(format!(
                "{name}: merged exit mismatch: interpreter {:?}, native {:?}",
                interp_output.status.code(),
                native_output.status.code()
            ));
        }
        if interp_output.status.code() != Some(exit) {
            failures.push(format!(
                "{name}: interpreter merged exit {:?}, expected {exit}",
                interp_output.status.code()
            ));
        }
        if native_output.status.code() != Some(exit) {
            failures.push(format!(
                "{name}: native merged exit {:?}, expected {exit}",
                native_output.status.code()
            ));
        }
        if interp_output.bytes != native_output.bytes {
            failures.push(format!(
                "{name}: merged stream differs between engines\ninterpreter:\n{}\nnative:\n{}",
                show_bytes(&interp_output.bytes),
                show_bytes(&native_output.bytes)
            ));
        }
        if interp_output.bytes != expected {
            failures.push(format!(
                "{name}: interpreter merged stream mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&interp_output.bytes)
            ));
        }
        if native_output.bytes != expected {
            failures.push(format!(
                "{name}: native merged stream mismatch\nexpected:\n{}\nactual:\n{}",
                show_bytes(&expected),
                show_bytes(&native_output.bytes)
            ));
        }
    }
}

#[test]
fn interpreter_and_native_are_byte_identical_for_every_run_golden() {
    if skip_without_cc("differential run goldens") {
        return;
    }
    // Safe: the only test in this binary, so no other thread reads the env.
    std::env::set_var("ALEPH_CFLAGS", "-Wall -Wextra -Werror");

    let mut failures = Vec::new();
    for aph in run_cases() {
        check_case(&aph, &mut failures);
    }

    assert!(
        failures.is_empty(),
        "{} differential failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
