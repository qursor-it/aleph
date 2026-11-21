//! Node-level edits for `aleph edit` (ADR-0029).
//!
//! Edits address nodes of the *input* AST by id (`n0`, `n1`, ...). They are
//! all resolved against that snapshot, rendered into new text, reparsed and
//! printed canonically. A result that does not parse is rejected (X010).

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::ast::{self, ExprKind, File};
use crate::diag::Diagnostic;
use crate::lexer::{self, TokenKind};
use crate::{builtins, parser, printer};

/// One edit as written in the `EDITS` JSON array.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEdit {
    op: String,
    node: String,
    // `with` lets a diagnostic's `fix` object (ADR-0028) be passed as is.
    #[serde(default, alias = "with")]
    text: Option<String>,
    #[serde(default)]
    position: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    File,
    Use,
    Type,
    Fn,
    Example,
    Stmt,
    Expr,
    Other,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::File => "file",
            Kind::Use => "use",
            Kind::Type => "type",
            Kind::Fn => "fn",
            Kind::Example => "ex",
            Kind::Stmt => "stmt",
            Kind::Expr => "expr",
            Kind::Other => "other",
        }
    }

    /// Kinds whose replacement text is one body line or part of one.
    fn single_line(self) -> bool {
        matches!(self, Kind::Example | Kind::Stmt | Kind::Expr)
    }
}

/// A node of the input AST: its kind and the end (exclusive) of its
/// pre-order id range. Pre-order ids make every subtree a contiguous range.
#[derive(Debug, Clone, Copy)]
struct Node {
    kind: Kind,
    end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Before,
    After,
    Start,
    End,
}

#[derive(Debug)]
enum Op {
    Replace(String),
    Delete,
    Insert(Position, String),
}

#[derive(Debug)]
struct Edit {
    node: u32,
    op: Op,
}

impl Edit {
    fn destructive(&self) -> bool {
        !matches!(self.op, Op::Insert(..))
    }
}

/// Apply `edits_json` to `source` and return the new canonical text.
pub fn apply(file_name: &str, source: &str, edits_json: &str) -> Result<String, Vec<Diagnostic>> {
    let output = parser::parse(file_name, source);
    if !output.diagnostics.is_empty() {
        return Err(output.diagnostics);
    }
    let Some(file) = output.file else {
        return Err(vec![Diagnostic::not_implemented(file_name)]);
    };

    let raw: Vec<RawEdit> = serde_json::from_str(edits_json)
        .map_err(|err| vec![invalid(file_name, err.to_string())])?;
    let nodes = index(&file);
    let edits = raw
        .into_iter()
        .map(|edit| resolve(file_name, &nodes, edit))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|diagnostic| vec![*diagnostic])?;
    check_conflicts(file_name, &nodes, &edits)?;

    let marked = render(&file, &nodes, &edits);
    let fragments: BTreeMap<u32, &str> = edits
        .iter()
        .filter_map(|edit| match &edit.op {
            Op::Replace(text) if nodes[&edit.node].kind == Kind::Expr => {
                Some((edit.node, text.as_str()))
            }
            _ => None,
        })
        .collect();
    let (text, _) = substitute(&marked, &fragments, |_, fragment| fragment.to_string());
    let reparsed = parser::parse(file_name, &text);
    match (reparsed.file, reparsed.diagnostics.is_empty()) {
        (Some(edited), true) => {
            check_fragments(file_name, &file, &marked, &fragments)?;
            Ok(printer::print_file(&edited))
        }
        _ => Err(vec![Diagnostic::error(
            "X010",
            file_name,
            0,
            0,
            "edit does not parse",
        )
        .with_expected_found(
            "parsable program",
            first_found(&reparsed.diagnostics),
        )]),
    }
}

fn invalid(file_name: &str, found: impl Into<String>) -> Diagnostic {
    Diagnostic::error("X007", file_name, 0, 0, "invalid edits json")
        .with_expected_found("[{op,node,text?,position?}]", found)
}

fn node_error(file_name: &str, code: &str, msg: &str, node: u32) -> Box<Diagnostic> {
    Box::new(Diagnostic::error(code, file_name, 0, 0, msg).with_node(format!("n{node}")))
}

/// Kind and id range of every node of the input AST.
fn index(file: &File) -> HashMap<u32, Node> {
    let total = ast::collect_ids(file).len() as u32;
    let mut nodes: HashMap<u32, Node> = (0..total)
        .map(|id| {
            let node = Node {
                kind: Kind::Other,
                end: id + 1,
            };
            (id, node)
        })
        .collect();
    let mut set = |id: u32, kind: Kind, end: u32| {
        nodes.insert(id, Node { kind, end });
    };

    set(file.id.0, Kind::File, total);
    for item in &file.uses {
        set(item.id.0, Kind::Use, item.id.0 + 1);
    }
    // Item ranges end where the next item (or the file) starts.
    let starts: Vec<u32> = file
        .types
        .iter()
        .map(|item| item.id.0)
        .chain(file.fns.iter().map(|item| item.id.0))
        .chain([total])
        .collect();
    for (k, item) in file.types.iter().enumerate() {
        set(item.id.0, Kind::Type, starts[k + 1]);
    }
    let offset = file.types.len();
    for (k, function) in file.fns.iter().enumerate() {
        set(function.id.0, Kind::Fn, starts[offset + k + 1]);
        let lines: Vec<u32> = function
            .examples
            .iter()
            .map(|example| example.id.0)
            .chain(function.stmts.iter().map(|stmt| stmt.id.0))
            .chain([function.result.id.0])
            .collect();
        for pair in lines.windows(2) {
            let kind = if function.examples.iter().any(|e| e.id.0 == pair[0]) {
                Kind::Example
            } else {
                Kind::Stmt
            };
            set(pair[0], kind, pair[1]);
        }
        for expr in fn_exprs(function) {
            // shortcut: quadratic in expression depth, fine for source files.
            ast::walk_expr(expr, &mut |sub| {
                let mut end = sub.id.0 + 1;
                ast::walk_expr(sub, &mut |inner| end = end.max(inner.id.0 + 1));
                set(sub.id.0, Kind::Expr, end);
            });
        }
    }
    nodes
}

fn fn_exprs(function: &ast::Fn) -> Vec<&ast::Expr> {
    let mut exprs = Vec::new();
    for example in &function.examples {
        exprs.push(&example.lhs);
        exprs.push(&example.rhs);
    }
    for stmt in &function.stmts {
        match &stmt.kind {
            ast::StmtKind::Let { value, .. } | ast::StmtKind::Do { value } => exprs.push(value),
            ast::StmtKind::Guard { cond, value } => {
                exprs.push(cond);
                exprs.push(value);
            }
        }
    }
    exprs.push(&function.result);
    exprs
}

fn resolve(
    file_name: &str,
    nodes: &HashMap<u32, Node>,
    raw: RawEdit,
) -> Result<Edit, Box<Diagnostic>> {
    let id = raw
        .node
        .strip_prefix('n')
        // Canonical spelling only: `n0`, `n13`, never `n013`.
        .filter(|digits| {
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == &"0" || !digits.starts_with('0'))
        })
        .ok_or_else(|| Box::new(invalid(file_name, format!("node {}", raw.node))))?;
    // Well spelled but beyond any id this file can have.
    let id: u32 = id.parse().map_err(|_| {
        Box::new(
            Diagnostic::error("X008", file_name, 0, 0, "unknown node id")
                .with_node(raw.node.clone()),
        )
    })?;
    let kind = nodes
        .get(&id)
        .ok_or_else(|| node_error(file_name, "X008", "unknown node id", id))?
        .kind;
    let not_allowed = || {
        let mut diagnostic = node_error(file_name, "X009", "edit not allowed for node kind", id);
        diagnostic.found = Some(format!("{} {}", raw.op, kind.name()));
        diagnostic
    };

    let op = match raw.op.as_str() {
        "replace" => {
            if kind == Kind::Other {
                return Err(not_allowed());
            }
            if raw.position.is_some() {
                return Err(Box::new(invalid(file_name, "position on replace")));
            }
            let blank_ok = kind == Kind::File;
            Op::Replace(required_text(
                file_name,
                raw.text,
                kind.single_line(),
                blank_ok,
                id,
            )?)
        }
        "delete" => {
            if !matches!(
                kind,
                Kind::Use | Kind::Type | Kind::Fn | Kind::Example | Kind::Stmt
            ) {
                return Err(not_allowed());
            }
            if raw.text.is_some() || raw.position.is_some() {
                return Err(Box::new(invalid(file_name, "text or position on delete")));
            }
            Op::Delete
        }
        "insert" => {
            let position = match (kind, raw.position.as_deref()) {
                (Kind::File, None) => Position::End,
                (Kind::Fn, None | Some("end")) => Position::End,
                (Kind::Fn, Some("start")) => Position::Start,
                (Kind::Example | Kind::Stmt, None | Some("after")) => Position::After,
                (Kind::Example | Kind::Stmt, Some("before")) => Position::Before,
                (Kind::File | Kind::Fn | Kind::Example | Kind::Stmt, Some(other)) => {
                    return Err(Box::new(invalid(file_name, format!("position {other}"))));
                }
                _ => return Err(not_allowed()),
            };
            // Only an item inserted at `n0` may span lines; elsewhere it is a body line.
            let one_line = kind != Kind::File;
            Op::Insert(
                position,
                required_text(file_name, raw.text, one_line, false, id)?,
            )
        }
        other => return Err(Box::new(invalid(file_name, format!("op {other}")))),
    };
    Ok(Edit { node: id, op })
}

/// The edit's text; body-line and expression fragments must be one line.
/// The marker character never appears in valid source, so it is rejected.
/// Blank text (which would silently delete or do nothing) is allowed only
/// when replacing the whole file.
fn required_text(
    file_name: &str,
    text: Option<String>,
    one_line: bool,
    blank_ok: bool,
    id: u32,
) -> Result<String, Box<Diagnostic>> {
    let text = text.ok_or_else(|| Box::new(invalid(file_name, "missing text")))?;
    let problem = if one_line && text.contains('\n') {
        Some("newline in one-line text")
    } else if text.contains(MARK) {
        Some("U+0001 in text")
    } else if !blank_ok && text.trim().is_empty() {
        Some("blank text")
    } else {
        None
    };
    match problem {
        Some(found) => {
            let mut diagnostic = node_error(file_name, "X010", "edit does not parse", id);
            diagnostic.found = Some(found.to_string());
            Err(diagnostic)
        }
        None => Ok(text),
    }
}

/// X011 when an edit touches a node that another edit replaces or deletes.
fn check_conflicts(
    file_name: &str,
    nodes: &HashMap<u32, Node>,
    edits: &[Edit],
) -> Result<(), Vec<Diagnostic>> {
    let inside = |id: u32, outer: u32| id >= outer && id < nodes[&outer].end;
    for (j, later) in edits.iter().enumerate() {
        for earlier in &edits[..j] {
            let clash = (earlier.destructive() && inside(later.node, earlier.node))
                || (later.destructive() && inside(earlier.node, later.node));
            if clash {
                let mut diagnostic = node_error(file_name, "X011", "conflicting edits", later.node);
                diagnostic.found = Some(format!("n{}", earlier.node));
                return Err(vec![*diagnostic]);
            }
        }
    }
    Ok(())
}

// Control characters never appear in printed source (strings escape them).
const MARK: char = '\u{1}';

fn marker(id: u32) -> String {
    format!("{MARK}{id}{MARK}")
}

/// Replace every marker in `marked`, left to right, with `wrap(fragment)`.
/// Returns the text and the 1-based (line, byte col) where each replacement
/// starts.
fn substitute(
    marked: &str,
    fragments: &BTreeMap<u32, &str>,
    wrap: impl Fn(u32, &str) -> String,
) -> (String, Vec<(u32, u32)>) {
    let mut out = String::new();
    let mut starts = Vec::new();
    let mut rest = marked;
    while let Some(open) = rest.find(MARK) {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find(MARK).expect("markers come in pairs");
        let id: u32 = after[..close].parse().expect("a marker holds an id");
        let line = out.matches('\n').count() + 1;
        let col = out.len() - out.rfind('\n').map_or(0, |i| i + 1) + 1;
        starts.push((line as u32, col as u32));
        out.push_str(&wrap(id, fragments[&id]));
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    (out, starts)
}

/// Every expression fragment must be exactly one expression in place. A
/// balanced fragment wrapped as `[fragment]` must parse as a one-element list
/// (otherwise `7 8` would silently become two list elements). A fragment in
/// the function position of `map`/`filter`/`fold`/`map2` is probed as
/// `[CALLEE fragment 0..]`, because a `lam` only parses there and takes its
/// binder count from the callee (ADR-0037).
fn check_fragments(
    file_name: &str,
    file: &File,
    marked: &str,
    fragments: &BTreeMap<u32, &str>,
) -> Result<(), Vec<Diagnostic>> {
    if fragments.is_empty() {
        return Ok(());
    }
    let not_one = |found: String| {
        vec![
            Diagnostic::error("X010", file_name, 0, 0, "edit does not parse")
                .with_expected_found("one expression", found),
        ]
    };
    for (id, fragment) in fragments {
        if !balanced(fragment) {
            return Err(not_one(format!("n{id} unbalanced brackets")));
        }
    }
    let callees = fn_arg_callees(file);
    let (probe, starts) = substitute(marked, fragments, |id, fragment| match callees.get(&id) {
        Some((callee, arity)) => format!("[{callee} {fragment}{}]", " 0".repeat(arity - 1)),
        None => format!("[{fragment}]"),
    });
    let output = parser::parse(file_name, &probe);
    let Some(file) = output.file.filter(|_| output.diagnostics.is_empty()) else {
        // Positions in the bracketed probe text would mislead: code and message only.
        let found = output
            .diagnostics
            .first()
            .map(|d| format!("{} {}", d.code, d.msg))
            .unwrap_or_default();
        return Err(not_one(found));
    };
    let mut singles = 0;
    for_each_expr(&file, &mut |expr| {
        if let ExprKind::List { elems } = &expr.kind {
            if elems.len() == 1 && starts.contains(&(expr.span.line, expr.span.col)) {
                singles += 1;
            }
        }
    });
    if singles == starts.len() {
        Ok(())
    } else {
        Err(not_one("several expressions".to_string()))
    }
}

/// Brackets in the fragment's tokens never close more than they opened and
/// end balanced; lexical errors count as unbalanced.
fn balanced(fragment: &str) -> bool {
    let (lines, diagnostics) = lexer::lex("", fragment);
    let mut depth = 0i64;
    for token in lines.iter().flat_map(|line| &line.tokens) {
        match token.kind {
            TokenKind::LBracket => depth += 1,
            TokenKind::RBracket => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    diagnostics.is_empty() && depth == 0
}

/// Ids of the function arguments of higher-order builtin calls, with the
/// callee and its arity.
fn fn_arg_callees(file: &File) -> HashMap<u32, (&'static str, usize)> {
    let mut callees = HashMap::new();
    for_each_expr(file, &mut |expr| {
        if let ExprKind::Call {
            callee,
            callee_kind: ast::CalleeKind::Builtin,
            args,
        } = &expr.kind
        {
            let spec = builtins::lookup(callee);
            if let (Some(spec), Some(first)) = (spec, args.first()) {
                if matches!(spec.params.first(), Some(builtins::Ty::Fn(..))) {
                    callees.insert(first.id.0, (spec.name, spec.arity()));
                }
            }
        }
    });
    callees
}

fn for_each_expr(file: &File, visit: &mut impl FnMut(&ast::Expr)) {
    for expr in file.fns.iter().flat_map(fn_exprs) {
        ast::walk_expr(expr, visit);
    }
}

fn first_found(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .first()
        .map(|d| format!("{} {}:{} {}", d.code, d.line, d.col, d.msg))
        .unwrap_or_default()
}

/// Render the edited, not yet canonical, source text.
fn render(file: &File, nodes: &HashMap<u32, Node>, edits: &[Edit]) -> String {
    let mut replaced: HashMap<u32, &str> = HashMap::new();
    let mut deleted: Vec<u32> = Vec::new();
    let mut inserts: Vec<(u32, Position, &str)> = Vec::new();
    for edit in edits {
        match &edit.op {
            Op::Replace(text) => {
                replaced.insert(edit.node, text);
            }
            Op::Delete => deleted.push(edit.node),
            Op::Insert(position, text) => inserts.push((edit.node, *position, text)),
        }
    }
    if let Some(text) = replaced.get(&file.id.0) {
        return text.to_string();
    }

    // Expression replacements become markers, substituted after printing.
    let mut marked = file.clone();
    for function in &mut marked.fns {
        let mut visit = |expr: &mut ast::Expr| {
            if nodes[&expr.id.0].kind == Kind::Expr && replaced.contains_key(&expr.id.0) {
                expr.kind = ExprKind::Var {
                    name: marker(expr.id.0),
                };
            }
        };
        for example in &mut function.examples {
            ast::walk_expr_mut(&mut example.lhs, &mut visit);
            ast::walk_expr_mut(&mut example.rhs, &mut visit);
        }
        for stmt in &mut function.stmts {
            match &mut stmt.kind {
                ast::StmtKind::Let { value, .. } | ast::StmtKind::Do { value } => {
                    ast::walk_expr_mut(value, &mut visit)
                }
                ast::StmtKind::Guard { cond, value } => {
                    ast::walk_expr_mut(cond, &mut visit);
                    ast::walk_expr_mut(value, &mut visit);
                }
            }
        }
        ast::walk_expr_mut(&mut function.result, &mut visit);
    }

    let lines_at = |id: u32, position: Position| -> Vec<String> {
        inserts
            .iter()
            .filter(|(node, at, _)| *node == id && *at == position)
            .map(|(_, _, text)| format!(" {}", text.trim_start()))
            .collect()
    };
    let item = |id: u32, items: &mut Vec<String>, default: &dyn Fn() -> String| {
        if deleted.contains(&id) {
            return;
        }
        match replaced.get(&id) {
            Some(text) => items.push(text.trim_end_matches('\n').to_string()),
            None => items.push(default()),
        }
    };

    let mut items: Vec<String> = Vec::new();
    for use_item in &marked.uses {
        item(use_item.id.0, &mut items, &|| use_item.canonical_text());
    }
    for type_item in &marked.types {
        item(type_item.id.0, &mut items, &|| {
            printer::print_typedef(type_item)
        });
    }
    for function in &marked.fns {
        item(function.id.0, &mut items, &|| {
            let printed = printer::print_fn(function);
            let header = 1 + usize::from(function.doc.is_some());
            let mut out: Vec<String> = printed[..header].to_vec();
            out.extend(lines_at(function.id.0, Position::Start));
            let body_ids = function
                .examples
                .iter()
                .map(|example| example.id.0)
                .chain(function.stmts.iter().map(|stmt| stmt.id.0));
            for (k, id) in body_ids.enumerate() {
                out.extend(lines_at(id, Position::Before));
                if !deleted.contains(&id) {
                    match replaced.get(&id) {
                        Some(text) => out.push(format!(" {}", text.trim_start())),
                        None => out.push(printed[header + k].clone()),
                    }
                }
                out.extend(lines_at(id, Position::After));
            }
            out.extend(lines_at(function.id.0, Position::End));
            out.push(printed[printed.len() - 1].clone());
            out.join("\n")
        });
    }
    for (node, _, text) in &inserts {
        if *node == file.id.0 {
            items.push(text.trim_end_matches('\n').to_string());
        }
    }

    let mut text = items.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "fn inc x:i64 -> i64\n ex inc 1 = 2\n let y + x 1\n y\n";

    fn ok(edits: &str) -> String {
        apply("t.aph", SRC, edits).expect("edit applies")
    }

    fn code(edits: &str) -> String {
        apply("t.aph", SRC, edits).expect_err("edit fails")[0]
            .code
            .clone()
    }

    #[test]
    fn ids_and_ranges_follow_pre_order() {
        // n0 file, n1 fn, n2 param, n3 type, n4 ret, n5 ex, n6..n7 lhs, n8 rhs,
        // n9 let, n10..n12 value, n13 result.
        let file = parser::parse("t.aph", SRC).file.unwrap();
        let nodes = index(&file);
        assert_eq!(nodes[&0].kind, Kind::File);
        assert_eq!(nodes[&1].kind, Kind::Fn);
        assert_eq!(nodes[&2].kind, Kind::Other);
        assert_eq!(nodes[&5].kind, Kind::Example);
        assert_eq!(nodes[&5].end, 9);
        assert_eq!(nodes[&9].kind, Kind::Stmt);
        assert_eq!(nodes[&9].end, 13);
        assert_eq!(nodes[&10].kind, Kind::Expr);
        assert_eq!(nodes[&10].end, 13);
        assert_eq!(nodes[&11].end, 12);
        assert_eq!(nodes[&13].end, 14);
    }

    #[test]
    fn replaces_expressions_and_lines() {
        assert_eq!(
            ok(r#"[{"op":"replace","node":"n10","text":"* x 2"}]"#),
            "fn inc x:i64 -> i64\n ex inc 1 = 2\n let y * x 2\n y\n"
        );
        assert_eq!(
            ok(r#"[{"op":"replace","node":"n5","text":"ex inc 2 = 3"}]"#),
            "fn inc x:i64 -> i64\n ex inc 2 = 3\n let y + x 1\n y\n"
        );
    }

    #[test]
    fn inserts_and_deletes_lines_and_items() {
        assert_eq!(
            ok(
                r#"[{"op":"insert","node":"n9","position":"before","text":"let z x"},
                   {"op":"replace","node":"n11","text":"z"}]"#
            ),
            "fn inc x:i64 -> i64\n ex inc 1 = 2\n let z x\n let y + z 1\n y\n"
        );
        assert_eq!(
            ok(r#"[{"op":"delete","node":"n9"},{"op":"replace","node":"n13","text":"+ x 1"}]"#),
            "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 1\n"
        );
        // Inserted items are put into canonical order.
        assert_eq!(
            ok(r#"[{"op":"insert","node":"n0","text":"fn dec x:i64 -> i64\n ex dec 2 = 1\n - x 1"}]"#),
            "fn dec x:i64 -> i64\n ex dec 2 = 1\n - x 1\nfn inc x:i64 -> i64\n ex inc 1 = 2\n let y + x 1\n y\n"
        );
        assert_eq!(
            ok(r#"[{"op":"insert","node":"n1","position":"start","text":"ex inc 0 = 1"}]"#),
            "fn inc x:i64 -> i64\n ex inc 0 = 1\n ex inc 1 = 2\n let y + x 1\n y\n"
        );
    }

    #[test]
    fn empty_edits_canonicalize() {
        assert_eq!(ok("[]"), SRC);
    }

    #[test]
    fn deleting_every_item_gives_the_empty_module() {
        assert_eq!(ok(r#"[{"op":"delete","node":"n1"}]"#), "");
        assert_eq!(
            apply(
                "t.aph",
                "",
                r#"[{"op":"insert","node":"n0","text":"use env"}]"#
            )
            .unwrap(),
            "use env\n"
        );
    }

    #[test]
    fn reports_edit_errors() {
        assert_eq!(code("{"), "X007");
        assert_eq!(code(r#"[{"op":"move","node":"n1"}]"#), "X007");
        assert_eq!(code(r#"[{"op":"replace","node":"x1","text":"1"}]"#), "X007");
        assert_eq!(
            code(r#"[{"op":"replace","node":"n013","text":"1"}]"#),
            "X007"
        );
        assert_eq!(
            code(r#"[{"op":"replace","node":"n4294967296","text":"1"}]"#),
            "X008"
        );
        // Blank text would silently delete or do nothing.
        assert_eq!(code(r#"[{"op":"replace","node":"n5","text":""}]"#), "X010");
        assert_eq!(
            code(r#"[{"op":"replace","node":"n1","text":"   "}]"#),
            "X010"
        );
        assert_eq!(code(r#"[{"op":"insert","node":"n1","text":""}]"#), "X010");
        assert_eq!(code(r#"[{"op":"replace","node":"n10"}]"#), "X007");
        assert_eq!(code(r#"[{"op":"delete","node":"n9","extra":1}]"#), "X007");
        assert_eq!(
            code(r#"[{"op":"replace","node":"n99","text":"1"}]"#),
            "X008"
        );
        assert_eq!(code(r#"[{"op":"delete","node":"n10"}]"#), "X009");
        assert_eq!(
            code(r#"[{"op":"replace","node":"n2","text":"y:i64"}]"#),
            "X009"
        );
        assert_eq!(
            code(r#"[{"op":"replace","node":"n10","text":"+ x"}]"#),
            "X010"
        );
        assert_eq!(
            code(r#"[{"op":"replace","node":"n10","text":"1\n2"}]"#),
            "X010"
        );
        assert_eq!(
            code(
                r#"[{"op":"replace","node":"n9","text":"let y 1"},{"op":"replace","node":"n11","text":"2"}]"#
            ),
            "X011"
        );
        assert_eq!(
            code(r#"[{"op":"delete","node":"n9"},{"op":"insert","node":"n9","text":"let z 1"}]"#),
            "X011"
        );
        assert_eq!(
            code(
                r#"[{"op":"replace","node":"n0","text":""},{"op":"insert","node":"n0","text":"use env"}]"#
            ),
            "X011"
        );
    }

    #[test]
    fn replaces_a_fn_with_several_lines() {
        assert_eq!(
            ok(
                r#"[{"op":"replace","node":"n1","text":"fn inc x:i64 -> i64\n ex inc 1 = 3\n + x 2"}]"#
            ),
            "fn inc x:i64 -> i64\n ex inc 1 = 3\n + x 2\n"
        );
        // A line inserted into a fn is still one line.
        assert_eq!(
            code(r#"[{"op":"insert","node":"n1","text":"let a 1\n let b 2"}]"#),
            "X010"
        );
    }

    #[test]
    fn fragments_are_one_expression() {
        let list = "fn l -> list i64\n ex l = [1 2 3]\n [1 2 3]\n";
        let run = |edits: &str| apply("t.aph", list, edits);
        // n9 is `3` in the example's rhs list.
        assert_eq!(
            run(r#"[{"op":"replace","node":"n9","text":"7 8"}]"#).unwrap_err()[0].code,
            "X010"
        );
        assert_eq!(
            run(r#"[{"op":"replace","node":"n9","text":"3] [4"}]"#).unwrap_err()[0].code,
            "X010"
        );
        assert_eq!(
            run(r#"[{"op":"replace","node":"n9","text":"+ 3 4"}]"#).unwrap(),
            "fn l -> list i64\n ex l = [1 2 + 3 4]\n [1 2 3]\n"
        );
        // A list fragment is one expression (types are `check`'s job).
        assert_eq!(
            run(r#"[{"op":"replace","node":"n9","text":"[3]"}]"#).unwrap(),
            "fn l -> list i64\n ex l = [1 2 [3]]\n [1 2 3]\n"
        );
    }

    #[test]
    fn fix_objects_are_edits() {
        assert_eq!(
            ok(r#"[{"op":"replace","node":"n10","with":"* x 2"}]"#),
            "fn inc x:i64 -> i64\n ex inc 1 = 2\n let y * x 2\n y\n"
        );
    }

    #[test]
    fn marker_characters_are_rejected() {
        assert_eq!(
            code(
                r#"[{"op":"replace","node":"n12","text":"\u000111\u0001"},{"op":"replace","node":"n11","text":"x"}]"#
            ),
            "X010"
        );
    }

    #[test]
    fn inserts_at_one_anchor_keep_their_order() {
        assert_eq!(
            ok(
                r#"[{"op":"insert","node":"n9","text":"let a 1"},{"op":"insert","node":"n9","text":"let b 2"}]"#
            ),
            "fn inc x:i64 -> i64\n ex inc 1 = 2\n let y + x 1\n let a 1\n let b 2\n y\n"
        );
        let doc = "fn inc x:i64 -> i64\n doc \"add one\"\n ex inc 1 = 2\n + x 1\n";
        assert_eq!(
            apply(
                "t.aph",
                doc,
                r#"[{"op":"insert","node":"n1","position":"start","text":"ex inc 0 = 1"}]"#
            )
            .unwrap(),
            "fn inc x:i64 -> i64\n doc \"add one\"\n ex inc 0 = 1\n ex inc 1 = 2\n + x 1\n"
        );
        assert_eq!(code(r#"[{"op":"insert","node":"n10","text":"1"}]"#), "X009");
    }

    #[test]
    fn lambda_bodies_and_whole_lambdas_are_expression_nodes() {
        // n9 map call, n10 lam, n11 body `+ x k`, n12 x, n13 k, n14 xs.
        let src = "fn f xs:list i64 k:i64 -> list i64\n map lam x + x k xs\n";
        let edit = |edits: &str| apply("t.aph", src, edits);
        let nodes = index(&parser::parse("t.aph", src).file.expect("parses"));
        assert_eq!((nodes[&10].kind, nodes[&10].end), (Kind::Expr, 14));
        assert_eq!((nodes[&11].kind, nodes[&11].end), (Kind::Expr, 14));
        assert_eq!(
            edit(r#"[{"op":"replace","node":"n11","text":"* x k"}]"#).expect("body"),
            "fn f xs:list i64 k:i64 -> list i64\n map lam x * x k xs\n"
        );
        assert_eq!(
            edit(r#"[{"op":"replace","node":"n10","text":"lam y - y k"}]"#).expect("lam"),
            "fn f xs:list i64 k:i64 -> list i64\n map lam y - y k xs\n"
        );
        assert_eq!(
            edit(r#"[{"op":"replace","node":"n10","text":"@abs"}]"#).expect("fn ref"),
            "fn f xs:list i64 k:i64 -> list i64\n map @abs xs\n"
        );
        // Two binders where map takes one, or a lam outside a function argument.
        let error = |edits: &str| edit(edits).expect_err("rejected")[0].code.clone();
        assert_eq!(
            error(r#"[{"op":"replace","node":"n10","text":"lam y z 1"}]"#),
            "X010"
        );
        assert_eq!(
            error(r#"[{"op":"replace","node":"n12","text":"lam y y"}]"#),
            "X010"
        );
        // Editing inside a replaced lambda conflicts.
        assert_eq!(
            error(
                r#"[{"op":"replace","node":"n10","text":"@abs"},{"op":"replace","node":"n12","text":"k"}]"#
            ),
            "X011"
        );
        // In a list, extra tokens after the lambda would silently add an element.
        let list = "fn f xs:list i64 -> list list i64\n [map lam x x xs]\n";
        let error = apply(
            "t.aph",
            list,
            r#"[{"op":"replace","node":"n10","text":"lam y y [1]"}]"#,
        )
        .expect_err("several expressions");
        assert_eq!(error[0].found.as_deref(), Some("several expressions"));
    }

    #[test]
    fn input_parse_errors_are_returned() {
        let diagnostics = apply("t.aph", "fn\n", "[]").unwrap_err();
        assert!(diagnostics[0].code.starts_with('P') || diagnostics[0].code.starts_with('L'));
    }
}
