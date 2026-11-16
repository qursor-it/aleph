//! Capability checker (SPEC §3.1, §8, ADR-0008, ADR-0020).
//!
//! Modules declare filesystem prefixes with `use`. Calls to effectful builtins
//! are denied unless a `use` of the required capability exists (`C001`) and,
//! for literal path arguments, a declared prefix covers the path (`C002`).
//! [`covers`] is the single path-normalization rule shared with the runtime's
//! `R003` check.

use std::collections::BTreeSet;

use crate::ast::{self, escape_str, CalleeKind, Capability, Expr, ExprKind, NodeId, StmtKind};
use crate::builtins;
use crate::diag::Diagnostic;
use crate::lexicon;

/// Whether a declared path `prefix` covers `path` (ADR-0020).
///
/// Both arguments are normalized by splitting on `/` and dropping empty and
/// `.` segments. A `..` segment makes both the prefix and the path invalid,
/// so it never covers anything. An absolute path is covered only by an
/// absolute prefix; the relative prefix `""` or `"."` covers every relative
/// path. Otherwise the prefix segments must be a leading run of the path
/// segments, so `data` covers `data` and `data/in.txt` but not `database/x`.
///
/// `use` declarations reject the empty prefix (`C003`), so the whole working
/// directory must be written explicitly as `"."`.
pub fn covers(prefix: &str, path: &str) -> bool {
    let Some(prefix) = Normalized::parse(prefix) else {
        return false;
    };
    let Some(path) = Normalized::parse(path) else {
        return false;
    };
    if prefix.absolute != path.absolute {
        return false;
    }
    path.segments.starts_with(&prefix.segments)
}

/// The capabilities declared by a file, sorted and deduplicated.
///
/// The first element is the canonical capability spelling (e.g. `fs.read`);
/// the second is the declared path prefix, if any. `aleph explain` uses this
/// to list capabilities first (ADR-0008).
pub fn capabilities(ast: &ast::File) -> Vec<(String, Option<String>)> {
    let mut declared: Vec<(String, Option<String>)> = ast
        .uses
        .iter()
        .map(|item| (item.cap.spelling().to_string(), item.path.clone()))
        .collect();
    declared.sort();
    declared.dedup();
    declared
}

/// Check capabilities for a parsed, type-clean file (SPEC §8).
///
/// Diagnostics are sorted by source position.
pub fn check_caps(file: &str, ast: &ast::File) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for item in &ast.uses {
        if invalid_use(item) {
            diagnostics.push(use_diagnostic(file, item));
        }
    }

    for function in &ast.fns {
        for example in &function.examples {
            ast::walk_expr(&example.lhs, &mut |expr| {
                check_call(file, ast, expr, &mut diagnostics)
            });
            ast::walk_expr(&example.rhs, &mut |expr| {
                check_call(file, ast, expr, &mut diagnostics)
            });
        }
        for stmt in &function.stmts {
            match &stmt.kind {
                StmtKind::Let { value, .. } | StmtKind::Do { value } => {
                    ast::walk_expr(value, &mut |expr| {
                        check_call(file, ast, expr, &mut diagnostics)
                    });
                }
            }
        }
        ast::walk_expr(&function.result, &mut |expr| {
            check_call(file, ast, expr, &mut diagnostics)
        });
    }

    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.col));
    diagnostics
}

/// A normalized path or path prefix.
struct Normalized<'a> {
    absolute: bool,
    segments: Vec<&'a str>,
}

impl<'a> Normalized<'a> {
    /// Normalize `path`, or `None` when it contains a `..` segment.
    fn parse(path: &'a str) -> Option<Self> {
        let absolute = path.starts_with('/');
        let mut segments = Vec::new();
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => return None,
                other => segments.push(other),
            }
        }
        Some(Self { absolute, segments })
    }
}

/// Whether a `use` item is invalid (`C003`): an empty prefix, a prefix with a
/// `..` segment, or a filesystem capability without a path.
fn invalid_use(item: &ast::Use) -> bool {
    match item.path.as_deref() {
        Some("") => true,
        Some(path) => path.split('/').any(|segment| segment == ".."),
        None => matches!(item.cap, Capability::FsRead | Capability::FsWrite),
    }
}

fn use_diagnostic(file: &str, item: &ast::Use) -> Diagnostic {
    let (msg, found) = match item.path.as_deref() {
        Some(path) => ("invalid capability prefix", path),
        None => ("missing capability path", ""),
    };
    let mut diagnostic = Diagnostic::error("C003", file, item.span.line, item.span.col, msg)
        .with_node(node_name(item.id));
    diagnostic.found = Some(found.to_string());
    diagnostic
}

fn check_call(file: &str, ast: &ast::File, expr: &Expr, diagnostics: &mut Vec<Diagnostic>) {
    let ExprKind::Call {
        callee,
        callee_kind: CalleeKind::Builtin,
        args,
    } = &expr.kind
    else {
        return;
    };
    let Some(spec) = builtins::lookup(callee) else {
        return;
    };
    let Some(capability) = spec.capability else {
        return;
    };

    if !ast.uses.iter().any(|item| item.cap == capability) {
        let mut diagnostic = Diagnostic::error(
            "C001",
            file,
            expr.span.line,
            expr.span.col,
            "missing capability",
        )
        .with_node(node_name(expr.id));
        diagnostic.expected = Some(capability.spelling().to_string());
        if let Some(line) = literal_use_line(spec, args) {
            diagnostic = diagnostic.with_fix("insert", "n0", line);
        }
        diagnostics.push(diagnostic);
        return;
    }

    let declared = ast.uses.iter().filter(|item| item.cap == capability);
    let Some(index) = spec.path_arg else {
        return;
    };
    let Some(arg) = args.get(index) else {
        debug_assert!(
            false,
            "builtin `{callee}` has path_arg {index} but the call has {} args",
            args.len()
        );
        return;
    };
    let ExprKind::Str { value } = &arg.kind else {
        return;
    };

    let prefixes: Vec<&str> = declared.filter_map(|item| item.path.as_deref()).collect();
    if prefixes.iter().any(|prefix| covers(prefix, value)) {
        return;
    }

    let expected = sorted_prefixes(prefixes).join(" ");
    let mut diagnostic = Diagnostic::error(
        "C002",
        file,
        expr.span.line,
        expr.span.col,
        "path not covered by capability",
    )
    .with_node(node_name(expr.id))
    .with_expected_found(expected, value.clone());
    if let Some(line) = literal_use_line(spec, args) {
        diagnostic = diagnostic.with_fix("insert", "n0", line);
    }
    diagnostics.push(diagnostic);
}

/// The `use` line that would grant a literal, valid filesystem path, or `None`
/// when the call has no such path (ADR-0028).
fn literal_use_line(spec: &builtins::BuiltinSpec, args: &[Expr]) -> Option<String> {
    let capability = spec.capability?;
    if !matches!(capability, Capability::FsRead | Capability::FsWrite) {
        return None;
    }
    let arg = args.get(spec.path_arg?)?;
    let ExprKind::Str { value } = &arg.kind else {
        return None;
    };
    // SPEC §8: prefixes are relative to the working directory.
    if value.is_empty() || value.starts_with('/') || value.split('/').any(|segment| segment == "..")
    {
        return None;
    }
    Some(format!(
        "{} {} \"{}\"",
        lexicon::keyword("capability").expect("use"),
        capability.spelling(),
        escape_str(value)
    ))
}

fn sorted_prefixes(prefixes: Vec<&str>) -> Vec<String> {
    let sorted: BTreeSet<String> = prefixes.into_iter().map(str::to_string).collect();
    sorted.into_iter().collect()
}

fn node_name(id: NodeId) -> String {
    format!("n{}", id.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn parse_ok(source: &str) -> ast::File {
        let output = parser::parse("t.aph", source);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}\n{source}",
            output.diagnostics
        );
        output.file.expect("parsed file")
    }

    fn caps(source: &str) -> Vec<Diagnostic> {
        let file = parse_ok(source);
        check_caps("t.aph", &file)
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    #[test]
    fn covers_relative_prefixes() {
        assert!(covers("data", "data"));
        assert!(covers("data", "data/in.txt"));
        assert!(covers("data/sub", "data/sub/x"));
        assert!(!covers("data", "database/x"));
        assert!(!covers("data/sub", "data/subway/x"));
        assert!(!covers("data/sub", "data"));
        assert!(covers("data//./sub", "data/sub/x"));
        assert!(covers("data", "data/./x"));
        assert!(covers("", "data/x"));
        assert!(covers("", ""));
        assert!(covers(".", ""));
        assert!(covers(".", "data/x"));
        assert!(!covers("data", "data/../etc/x"));
        assert!(!covers("data/../x", "data/x"));
        assert!(!covers("..", ".."));
    }

    #[test]
    fn covers_absolute_prefixes() {
        assert!(covers("/", "/"));
        assert!(covers("/", "/data"));
        assert!(covers("/data", "/data"));
        assert!(covers("/data", "/data/x"));
        assert!(covers("/data/", "/data//x"));
        assert!(covers("//data", "/data/x"));
        assert!(!covers("/data", "/database/x"));
        assert!(!covers("/data", "data/x"));
        assert!(!covers("/data", "data"));
        assert!(!covers("data", "/data/x"));
        assert!(!covers("", "/data"));
        assert!(!covers(".", "/data"));
    }

    #[test]
    fn capabilities_are_sorted_and_deduplicated() {
        let file = parse_ok(
            "use fs.write \"b\"\nuse fs.read \"a\"\nuse fs.read \"a\"\nuse env\nfn main -> unit\n nil\n",
        );
        assert_eq!(
            capabilities(&file),
            vec![
                ("env".to_string(), None),
                ("fs.read".to_string(), Some("a".to_string())),
                ("fs.write".to_string(), Some("b".to_string())),
            ]
        );
    }

    #[test]
    fn c001_is_reported_for_a_read_without_a_use() {
        let diagnostics =
            caps("fn main -> res unit str\n let text read \"data/in.txt\"\n ok nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "missing capability");
        assert_eq!(diagnostic.expected.as_deref(), Some("fs.read"));
        assert_eq!(diagnostic.line, 2);
        assert_eq!(diagnostic.col, 11);
        assert_eq!(diagnostic.node.as_deref(), Some("n6"));
    }

    #[test]
    fn c001_is_reported_when_only_the_other_filesystem_capability_is_declared() {
        let diagnostics = caps(
            "use fs.read \"data\"\nfn main -> res unit str\n let r write \"data/out.txt\" \"x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C001"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("fs.write"));
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[0].col, 8);
    }

    #[test]
    fn literal_paths_get_a_certain_insert_fix() {
        let diagnostics =
            caps("fn main -> res unit str\n let text read \"data/in.txt\"\n ok nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
        let fix = diagnostics[0].fix.as_ref().expect("C001 fix");
        assert_eq!(fix.op, "insert");
        assert_eq!(fix.node, "n0");
        assert_eq!(fix.with, "use fs.read \"data/in.txt\"");

        let diagnostics = caps(
            "use fs.read \"data\"\nfn main -> res unit str\n let text read \"other/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        let fix = diagnostics[0].fix.as_ref().expect("C002 fix");
        assert_eq!(fix.op, "insert");
        assert_eq!(fix.node, "n0");
        assert_eq!(fix.with, "use fs.read \"other/x\"");

        let diagnostics =
            caps("fn main -> res unit str\n let r write \"out/x.txt\" \"hi\"\n ok nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
        assert_eq!(
            diagnostics[0].fix.as_ref().map(|fix| fix.with.as_str()),
            Some("use fs.write \"out/x.txt\"")
        );
    }

    #[test]
    fn invalid_or_unknown_literal_paths_get_no_fix() {
        for source in [
            "fn main -> res unit str\n let text read \"data/../x\"\n ok nil\n",
            "fn main -> res unit str\n let text read \"\"\n ok nil\n",
            "fn main -> res unit str\n let text read \"/etc/x\"\n ok nil\n",
            "fn main -> res unit str\n let p \"data/x\"\n let text read p\n ok nil\n",
        ] {
            let diagnostics = caps(source);
            assert_eq!(codes(&diagnostics), ["C001"], "{source}");
            assert!(diagnostics[0].fix.is_none(), "{source}");
        }

        let diagnostics = caps(
            "use fs.read \"data\"\nfn main -> res unit str\n let text read \"data/../etc/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert!(diagnostics[0].fix.is_none());
    }

    #[test]
    fn the_insert_fix_escapes_the_literal_path() {
        let diagnostics = caps("fn main -> res unit str\n let text read \"a\\\"b\\nc\"\n ok nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
        assert_eq!(
            diagnostics[0].fix.as_ref().map(|fix| fix.with.as_str()),
            Some("use fs.read \"a\\\"b\\nc\"")
        );
    }

    #[test]
    fn c001_does_not_also_report_c002() {
        let diagnostics = caps("fn main -> res unit str\n let text read \"outside/x\"\n ok nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
    }

    #[test]
    fn c002_is_reported_for_a_literal_outside_the_prefix() {
        let diagnostics = caps(
            "use fs.read \"data\"\nfn main -> res unit str\n let text read \"other/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "path not covered by capability");
        assert_eq!(diagnostic.expected.as_deref(), Some("data"));
        assert_eq!(diagnostic.found.as_deref(), Some("other/x"));
        assert_eq!(diagnostic.line, 3);
        assert_eq!(diagnostic.col, 11);
        assert_eq!(diagnostic.node.as_deref(), Some("n7"));
    }

    #[test]
    fn c002_expected_lists_all_declared_prefixes_sorted() {
        let diagnostics = caps(
            "use fs.read \"z\"\nuse fs.read \"a\"\nfn main -> res unit str\n let text read \"m/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("a z"));
    }

    #[test]
    fn c002_rejects_paths_containing_parent_segments() {
        let diagnostics = caps(
            "use fs.read \"data\"\nfn main -> res unit str\n let text read \"data/../etc/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("data"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("data/../etc/x"));
    }

    #[test]
    fn non_literal_paths_are_not_checked_statically() {
        let diagnostics = caps(
            "use fs.read \"data\"\nfn main -> res unit str\n let p \"elsewhere/x\"\n let text read p\n ok nil\n",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn covered_literals_produce_no_diagnostics() {
        let diagnostics = caps(
            "use fs.read \"data\"\nuse fs.write \"data/out\"\nfn main -> res unit str\n let text read \"data/in.txt\"\n let r write \"data/out/x.txt\" \"hi\"\n ok nil\n",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn c003_reports_a_prefix_with_a_parent_segment() {
        let diagnostics = caps("use fs.read \"../x\"\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["C003"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "invalid capability prefix");
        assert_eq!(diagnostic.found.as_deref(), Some("../x"));
        assert_eq!(diagnostic.line, 1);
        assert_eq!(diagnostic.col, 1);
        assert_eq!(diagnostic.node.as_deref(), Some("n1"));
    }

    #[test]
    fn c003_reports_an_empty_prefix() {
        let diagnostics = caps("use fs.read \"\"\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["C003"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "invalid capability prefix");
        assert_eq!(diagnostic.found.as_deref(), Some(""));
        assert_eq!(diagnostic.node.as_deref(), Some("n1"));
    }

    #[test]
    fn c003_reports_a_filesystem_use_without_a_path() {
        let diagnostics = caps("use fs.write\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["C003"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "missing capability path");
        assert_eq!(diagnostic.found.as_deref(), Some(""));
        assert_eq!(diagnostic.node.as_deref(), Some("n1"));
    }

    #[test]
    fn v02_capabilities_are_accepted_silently() {
        let diagnostics =
            caps("use net.host \"example.com\"\nuse env\nuse time\nfn main -> unit\n nil\n");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn calls_inside_examples_are_checked() {
        let diagnostics = caps(
            "use fs.read \"data\"\nfn f -> res str str\n ex f = read \"outside/x\"\n ok \"x\"\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[0].col, 9);
    }

    #[test]
    fn diagnostics_are_sorted_by_source_position() {
        let diagnostics = caps(
            "use fs.write\nuse fs.read \"..\"\nfn main -> res unit str\n let text read \"data/x\"\n let r write \"out/x\" \"x\"\n ok nil\n",
        );
        assert_eq!(
            codes(&diagnostics),
            ["C003", "C003", "C002", "C002"],
            "{diagnostics:?}"
        );
        let positions: Vec<(u32, u32)> = diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.line, diagnostic.col))
            .collect();
        let mut sorted = positions.clone();
        sorted.sort_unstable();
        assert_eq!(positions, sorted);
    }

    #[test]
    fn spec_12_second_program_passes_capability_checks() {
        let source = "use fs.read \"data\"\nfn words s:str -> i64\n ex words \"a b c\" = 3\n len split trim s \" \"\nfn main -> res unit str\n let text try read \"data/in.txt\"\n do print show words text\n ok nil\n";
        assert!(caps(source).is_empty());
    }
}
