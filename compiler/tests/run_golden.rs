//! Golden tests for `aleph run`: every `tests/aph/NAME.aph` that has a
//! `NAME.out` is executed and its stdout compared byte for byte. An optional
//! `NAME.exit` gives the expected exit code (default 0) and an optional
//! `NAME.err` gives the expected stderr (default empty).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

fn aleph(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aleph"))
        .current_dir(repo_root())
        .args(args)
        .output()
        .expect("failed to run aleph")
}

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

#[test]
fn run_golden_files_match() {
    let repo = repo_root();
    for aph in run_cases() {
        let relative = aph.strip_prefix(&repo).expect("case is inside the repo");
        let relative_text = relative.to_string_lossy();
        let output = aleph(&["run", relative_text.as_ref()]);

        let expected_code = match fs::read_to_string(aph.with_extension("exit")) {
            Ok(text) => text
                .trim()
                .parse::<i32>()
                .expect("NAME.exit contains an integer"),
            Err(_) => 0,
        };
        assert_eq!(
            output.status.code(),
            Some(expected_code),
            "{} exit code (stderr: {})",
            relative.display(),
            String::from_utf8_lossy(&output.stderr)
        );

        let expected_stdout = fs::read(aph.with_extension("out")).expect("read .out");
        assert_eq!(
            output.stdout,
            expected_stdout,
            "stdout mismatch for {}\nexpected:\n{}\nactual:\n{}",
            relative.display(),
            String::from_utf8_lossy(&expected_stdout),
            String::from_utf8_lossy(&output.stdout),
        );

        let err_path = aph.with_extension("err");
        if err_path.is_file() {
            let expected_stderr = fs::read(&err_path).expect("read .err");
            assert_eq!(
                output.stderr,
                expected_stderr,
                "stderr mismatch for {}\nexpected:\n{}\nactual:\n{}",
                relative.display(),
                String::from_utf8_lossy(&expected_stderr),
                String::from_utf8_lossy(&output.stderr),
            );
        } else {
            assert!(
                output.stderr.is_empty(),
                "{} wrote unexpected stderr: {}",
                relative.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn run_goldens_are_canonical_and_check_clean() {
    let repo = repo_root();
    for aph in run_cases() {
        let relative = aph.strip_prefix(&repo).expect("case is inside the repo");
        let relative = relative.to_string_lossy();
        let relative_text = relative.as_ref();

        let fmt = aleph(&["fmt", relative_text]);
        assert_eq!(
            fmt.status.code(),
            Some(0),
            "{} is not parseable (stderr: {})",
            relative,
            String::from_utf8_lossy(&fmt.stderr)
        );
        assert_eq!(
            fmt.stdout,
            fs::read(&aph).expect("read .aph"),
            "{} is not canonical",
            relative
        );

        let check = aleph(&["check", relative_text]);
        assert_eq!(
            check.status.code(),
            Some(0),
            "{} check exit (stdout: {})",
            relative,
            String::from_utf8_lossy(&check.stdout)
        );
        assert!(
            check.stdout.is_empty(),
            "{} check produced diagnostics: {}",
            relative,
            String::from_utf8_lossy(&check.stdout)
        );
        assert!(
            check.stderr.is_empty(),
            "{} check wrote stderr: {}",
            relative,
            String::from_utf8_lossy(&check.stderr)
        );
    }
}
