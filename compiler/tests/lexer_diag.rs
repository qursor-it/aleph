//! Golden tests for lexer diagnostics: every `tests/aph/NAME.aph` that has a
//! `NAME.diag` is checked with `aleph check` and stdout must match byte for
//! byte (ADR-0010, T-004).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

#[test]
fn lexer_diagnostics_match_golden_files() {
    let repo = repo_root();
    let directory = repo.join("tests/aph");
    let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("tests/aph exists")
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
        .filter(|path| path.with_extension("diag").is_file())
        .collect();
    cases.sort();

    assert!(!cases.is_empty(), "no .aph/.diag cases in {directory:?}");

    for aph in cases {
        let relative = aph.strip_prefix(&repo).expect("case is inside the repo");
        let output = Command::new(env!("CARGO_BIN_EXE_aleph"))
            .current_dir(&repo)
            .arg("check")
            .arg(relative)
            .output()
            .expect("failed to run aleph");

        assert_eq!(
            output.status.code(),
            Some(1),
            "{} should exit 1",
            relative.display()
        );
        assert!(
            output.stderr.is_empty(),
            "{} wrote stderr: {}",
            relative.display(),
            String::from_utf8_lossy(&output.stderr)
        );

        let expected = fs::read(aph.with_extension("diag")).expect("read .diag");
        if output.stdout != expected {
            panic!(
                "stdout mismatch for {}\nexpected:\n{}\nactual:\n{}",
                relative.display(),
                String::from_utf8_lossy(&expected),
                String::from_utf8_lossy(&output.stdout),
            );
        }
    }
}

#[test]
fn ast_golden_files_match() {
    let repo = repo_root();
    let directory = repo.join("tests/aph");
    let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("tests/aph exists")
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
        .filter(|path| path.with_extension("ast").is_file())
        .collect();
    cases.sort();

    assert!(!cases.is_empty(), "no .aph/.ast cases in {directory:?}");

    for aph in cases {
        let relative = aph.strip_prefix(&repo).expect("case is inside the repo");
        let output = Command::new(env!("CARGO_BIN_EXE_aleph"))
            .current_dir(&repo)
            .arg("ast")
            .arg(relative)
            .output()
            .expect("failed to run aleph");

        assert_eq!(
            output.status.code(),
            Some(0),
            "{} should exit 0",
            relative.display()
        );
        assert!(
            output.stderr.is_empty(),
            "{} wrote stderr: {}",
            relative.display(),
            String::from_utf8_lossy(&output.stderr)
        );

        let expected = fs::read(aph.with_extension("ast")).expect("read .ast");
        if output.stdout != expected {
            panic!(
                "AST mismatch for {}\nexpected:\n{}\nactual:\n{}",
                relative.display(),
                String::from_utf8_lossy(&expected),
                String::from_utf8_lossy(&output.stdout),
            );
        }
    }
}
