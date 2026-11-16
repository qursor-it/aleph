//! T-023: every `tests/aph/fix_*.aph` golden carries a certain `fix`, and
//! applying that fix textually (then canonicalizing) clears the diagnostic;
//! the same fix object applied with `aleph edit` gives the same text.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use alephc::{ast, parser, printer};
use common::{file_name, repo_root, TempDir};

fn fix_cases() -> Vec<PathBuf> {
    let directory = repo_root().join("tests/aph");
    let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("tests/aph exists")
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
        .filter(|path| file_name(path).starts_with("fix_") && path.with_extension("diag").is_file())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no fix_*.aph goldens in {directory:?}");
    cases
}

fn check(path: &Path) -> (i32, Vec<serde_json::Value>) {
    let output = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .arg("check")
        .arg(path)
        .output()
        .expect("failed to run aleph");
    let text = String::from_utf8(output.stdout).expect("diagnostics are UTF-8");
    let diagnostics = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("diagnostic is JSON"))
        .collect();
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code().expect("exit code"), diagnostics)
}

fn find_expr(file: &ast::File, id: u32) -> Option<ast::Expr> {
    let mut found = None;
    let mut visit = |expr: &ast::Expr| {
        if expr.id.0 == id {
            found = Some(expr.clone());
        }
    };
    for function in &file.fns {
        for example in &function.examples {
            ast::walk_expr(&example.lhs, &mut visit);
            ast::walk_expr(&example.rhs, &mut visit);
        }
        for stmt in &function.stmts {
            match &stmt.kind {
                ast::StmtKind::Let { value, .. } | ast::StmtKind::Do { value } => {
                    ast::walk_expr(value, &mut visit)
                }
            }
        }
        ast::walk_expr(&function.result, &mut visit);
    }
    found
}

fn offset_at(source: &str, line: u32, col: u32) -> usize {
    let mut offset = 0;
    for (index, text) in source.split_inclusive('\n').enumerate() {
        if index + 1 == line as usize {
            return offset + col as usize - 1;
        }
        offset += text.len();
    }
    panic!("line {line} is past the end of the source");
}

fn apply(source: &str, diagnostic: &serde_json::Value) -> String {
    let fix = diagnostic.get("fix").expect("diagnostic has a fix");
    let op = fix["op"].as_str().expect("fix op");
    let node = fix["node"].as_str().expect("fix node");
    let with = fix["with"].as_str().expect("fix text");
    match op {
        "insert" => {
            assert_eq!(node, "n0", "inserts target n0");
            format!("{with}\n{source}")
        }
        "replace" if diagnostic["code"] == "F001" => with.to_string(),
        "replace" => {
            let parsed = parser::parse("t.aph", source);
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            let file = parsed.file.expect("parsed file");
            let id = node
                .strip_prefix('n')
                .and_then(|digits| digits.parse::<u32>().ok())
                .expect("node id");
            let expr = find_expr(&file, id).unwrap_or_else(|| panic!("node {node} is an expr"));
            let old = printer::print_expr(&expr);
            let start = offset_at(
                source,
                diagnostic["line"].as_u64().expect("line") as u32,
                diagnostic["col"].as_u64().expect("col") as u32,
            );
            assert!(
                source[start..].starts_with(&old),
                "node {node} text `{old}` not found at line/col"
            );
            format!(
                "{}{}{}",
                &source[..start],
                with,
                &source[start + old.len()..]
            )
        }
        other => panic!("unknown fix op `{other}`"),
    }
}

fn canonicalize(source: &str) -> String {
    let parsed = parser::parse("t.aph", source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    printer::print_file(&parsed.file.expect("parsed file"))
}

#[test]
fn every_fix_golden_applies_and_clears_its_diagnostic() {
    let mut failures = Vec::new();
    for aph in fix_cases() {
        let name = file_name(&aph);
        let source = fs::read_to_string(&aph).expect("read fix golden");
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join(&name);
        fs::write(&path, &source).expect("write source");

        let (code, diagnostics) = check(&path);
        assert_eq!(code, 1, "{name}: failing golden must exit 1");
        if diagnostics.is_empty() {
            failures.push(format!("{name}: no diagnostic"));
            continue;
        }
        for diagnostic in &diagnostics {
            if diagnostic.get("fix").is_none() {
                failures.push(format!(
                    "{name}: {} has no fix: {diagnostic}",
                    diagnostic["code"]
                ));
                continue;
            }
            let failing_code = diagnostic["code"].as_str().expect("code").to_string();
            let applied = canonicalize(&apply(&source, diagnostic));
            // A fix object is also a valid `aleph edit` edit (ADR-0029).
            match alephc::edit::apply(&name, &source, &format!("[{}]", diagnostic["fix"])) {
                Ok(edited) if edited == applied => {}
                Ok(edited) => failures.push(format!(
                    "{name}: aleph edit gave a different result:\n{edited}"
                )),
                Err(errors) => {
                    failures.push(format!("{name}: aleph edit rejected the fix: {errors:?}"))
                }
            }
            fs::write(&path, &applied).expect("write applied source");
            let (after_code, after) = check(&path);
            if after_code != 0 {
                failures.push(format!(
                    "{name}: applying {} left exit {after_code}: {after:?}\n{applied}",
                    diagnostic["code"]
                ));
            }
            if after
                .iter()
                .any(|diagnostic| diagnostic["code"] == failing_code.as_str())
            {
                failures.push(format!(
                    "{name}: {} still present after applying its fix",
                    diagnostic["code"]
                ));
            }
            // Restore the failing source for the next diagnostic (one per golden).
            fs::write(&path, &source).expect("restore source");
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
