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

/// The host of an `http://` or `https://` URL (ADR-0038).
///
/// The host is the text after `//` up to the first `/`, `?`, `#` or `:`.
/// Another scheme, an empty host, or an `@` before the first `/`, `?` or `#`
/// gives `None` (userinfo is not allowed because a `@` there could hide the
/// real host from the capability check).
pub fn url_host(url: &str) -> Option<&str> {
    if url.contains('\0') {
        return None; // curl cannot receive it; Tlön rejects it too
    }
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.contains('@') {
        return None;
    }
    let host_end = authority.find(':').unwrap_or(authority.len());
    let host = &authority[..host_end];
    (!host.is_empty()).then_some(host)
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
    // The same `use` line is offered once, so applying every fix never
    // duplicates a grant.
    let mut offered = BTreeSet::new();
    for diagnostic in &mut diagnostics {
        if let Some(fix) = &diagnostic.fix {
            if !offered.insert(fix.with.clone()) {
                diagnostic.fix = None;
            }
        }
    }
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
/// `..` segment, a host with `/`, `:`, `@`, `?`, `#` or whitespace, or a
/// filesystem/network capability without a path.
fn invalid_use(item: &ast::Use) -> bool {
    match item.path.as_deref() {
        Some("") => true,
        Some(path) => match item.cap {
            Capability::FsRead | Capability::FsWrite => {
                path.split('/').any(|segment| segment == "..")
            }
            Capability::NetHost => path
                .chars()
                .any(|c| matches!(c, '/' | ':' | '@' | '?' | '#') || c.is_whitespace()),
            Capability::Env | Capability::Time => false,
        },
        None => matches!(
            item.cap,
            Capability::FsRead | Capability::FsWrite | Capability::NetHost
        ),
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

    if capability == Capability::NetHost {
        check_fetch_host(file, expr, declared, args, diagnostics);
        return;
    }

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

/// Check a literal `fetch` URL against the declared `net.host` values.
fn check_fetch_host<'a>(
    file: &str,
    expr: &Expr,
    declared: impl Iterator<Item = &'a ast::Use>,
    args: &[Expr],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(arg) = args.first() else {
        return;
    };
    let ExprKind::Str { value } = &arg.kind else {
        return;
    };
    let hosts: Vec<&str> = declared.filter_map(|item| item.path.as_deref()).collect();
    if let Some(host) = url_host(value) {
        if hosts
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(host))
        {
            return;
        }
    }

    let expected = sorted_prefixes(hosts).join(" ");
    let mut diagnostic = Diagnostic::error(
        "C002",
        file,
        expr.span.line,
        expr.span.col,
        "host not covered by capability",
    )
    .with_node(node_name(expr.id))
    .with_expected_found(expected, value.clone());
    if let Some(line) = literal_use_line(builtins::lookup("fetch").expect("fetch"), args) {
        diagnostic = diagnostic.with_fix("insert", "n0", line);
    }
    diagnostics.push(diagnostic);
}

/// The `use` line that would grant a literal, valid filesystem path or URL
/// host, or `None` when the call has no such argument (ADR-0028, ADR-0038).
fn literal_use_line(spec: &builtins::BuiltinSpec, args: &[Expr]) -> Option<String> {
    let capability = spec.capability?;
    let value = match capability {
        Capability::FsRead | Capability::FsWrite => {
            let arg = args.get(spec.path_arg?)?;
            let ExprKind::Str { value } = &arg.kind else {
                return None;
            };
            // SPEC §8: prefixes are relative to the working directory. A path
            // that normalizes to nothing (`.`, `./`) would grant the whole
            // directory.
            let normalized = Normalized::parse(value)?;
            if normalized.absolute || normalized.segments.is_empty() {
                return None;
            }
            value
        }
        Capability::NetHost => url_host(match &args.first()?.kind {
            ExprKind::Str { value } => value,
            _ => return None,
        })?,
        Capability::Env | Capability::Time => return None,
    };
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
    fn url_host_extracts_the_authority_host() {
        assert_eq!(url_host("http://example.com"), Some("example.com"));
        assert_eq!(url_host("https://example.com/a/b"), Some("example.com"));
        assert_eq!(url_host("http://example.com:8080/x"), Some("example.com"));
        assert_eq!(url_host("http://example.com?q=1"), Some("example.com"));
        assert_eq!(url_host("http://example.com#frag"), Some("example.com"));
        assert_eq!(url_host("HTTP://example.com"), None);
        assert_eq!(url_host("ftp://example.com"), None);
        assert_eq!(url_host("http://"), None);
        assert_eq!(url_host("http://:80/x"), None);
        assert_eq!(url_host("http://localhost:8080/ok"), Some("localhost"));
        assert_eq!(url_host("http://user@example.com/x"), None);
        assert_eq!(url_host("http://user:pass@example.com/x"), None);
        assert_eq!(url_host("http://example.com@evil.com/x"), None);
        assert_eq!(url_host("http://example.com/\0x"), None);
    }

    #[test]
    fn c001_is_reported_for_fetch_without_a_use() {
        let diagnostics = caps(
            "fn main -> res unit str\n let body try fetch \"http://example.com/ok\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C001"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "missing capability");
        assert_eq!(diagnostic.expected.as_deref(), Some("net.host"));
        assert_eq!(diagnostic.line, 2);
    }

    #[test]
    fn fetch_literals_get_a_certain_insert_fix() {
        let diagnostics = caps(
            "fn main -> res unit str\n let body try fetch \"http://Example.COM/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C001"]);
        assert_eq!(
            diagnostics[0].fix.as_ref().map(|fix| fix.with.as_str()),
            Some("use net.host \"Example.COM\"")
        );

        let diagnostics = caps(
            "use net.host \"example.com\"\nfn main -> res unit str\n let body try fetch \"http://other.com/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert_eq!(
            diagnostics[0].fix.as_ref().map(|fix| fix.with.as_str()),
            Some("use net.host \"other.com\"")
        );

        let diagnostics = caps(
            "fn main -> res unit str\n let body try fetch \"mailto:x@example.com\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C001"]);
        assert!(diagnostics[0].fix.is_none());
    }

    #[test]
    fn c002_is_reported_for_a_literal_url_with_another_host() {
        let diagnostics = caps(
            "use net.host \"example.com\"\nfn main -> res unit str\n let body try fetch \"http://other.com/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "host not covered by capability");
        assert_eq!(diagnostic.expected.as_deref(), Some("example.com"));
        assert_eq!(diagnostic.found.as_deref(), Some("http://other.com/x"));
        assert_eq!(diagnostic.line, 3);
    }

    #[test]
    fn c002_lists_all_declared_hosts_sorted_and_ignores_case_when_covered() {
        let diagnostics = caps(
            "use net.host \"z.example\"\nuse net.host \"a.example\"\nfn main -> res unit str\n let body try fetch \"http://m.example/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert_eq!(
            diagnostics[0].expected.as_deref(),
            Some("a.example z.example")
        );

        let diagnostics = caps(
            "use net.host \"EXAMPLE.com\"\nfn main -> res unit str\n let body try fetch \"http://example.COM/x\"\n ok nil\n",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn c002_is_reported_for_a_literal_url_without_a_host() {
        let diagnostics = caps(
            "use net.host \"example.com\"\nfn main -> res unit str\n let body try fetch \"ftp://example.com/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C002"]);
        assert_eq!(diagnostics[0].msg, "host not covered by capability");
        assert!(diagnostics[0].fix.is_none());
    }

    #[test]
    fn computed_fetch_urls_are_not_checked_statically() {
        let diagnostics = caps(
            "use net.host \"example.com\"\nfn main -> res unit str\n let url \"http://other.com/x\"\n let body try fetch url\n ok nil\n",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn c003_reports_invalid_net_host_values() {
        for value in ["example.com:80", "a/b", "a@b", "a?b", "a#b", "a b"] {
            let source = format!("use net.host \"{value}\"\nfn main -> unit\n nil\n");
            let diagnostics = caps(&source);
            assert_eq!(codes(&diagnostics), ["C003"], "{value}");
            assert_eq!(diagnostics[0].found.as_deref(), Some(value));
        }
        let diagnostics = caps("use net.host\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["C003"]);
        assert_eq!(diagnostics[0].msg, "missing capability path");
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
    fn c001_is_reported_for_env_without_a_use() {
        let diagnostics = caps("fn main -> unit\n do print show has env \"PATH\"\n nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "missing capability");
        assert_eq!(diagnostic.expected.as_deref(), Some("env"));
        assert_eq!(diagnostic.line, 2);
        assert!(diagnostic.fix.is_none());
    }

    #[test]
    fn c001_is_reported_for_now_without_a_use() {
        let diagnostics = caps("fn main -> unit\n do print show > now 1700000000000\n nil\n");
        assert_eq!(codes(&diagnostics), ["C001"]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.msg, "missing capability");
        assert_eq!(diagnostic.expected.as_deref(), Some("time"));
        assert_eq!(diagnostic.line, 2);
        assert!(diagnostic.fix.is_none());
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
    fn the_same_use_line_is_offered_once() {
        let diagnostics = caps(
            "fn main -> res unit str\n let a read \"data/x\"\n let b read \"data/x\"\n ok nil\n",
        );
        assert_eq!(codes(&diagnostics), ["C001", "C001"]);
        assert!(diagnostics[0].fix.is_some());
        assert!(diagnostics[1].fix.is_none());
    }

    #[test]
    fn invalid_or_unknown_literal_paths_get_no_fix() {
        for source in [
            "fn main -> res unit str\n let text read \"data/../x\"\n ok nil\n",
            "fn main -> res unit str\n let text read \"\"\n ok nil\n",
            "fn main -> res unit str\n let text read \"/etc/x\"\n ok nil\n",
            // These normalize to the whole working directory.
            "fn main -> res unit str\n let text read \".\"\n ok nil\n",
            "fn main -> res unit str\n let text read \"./\"\n ok nil\n",
            "fn main -> res unit str\n let text read \"./.\"\n ok nil\n",
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
