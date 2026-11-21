//! `aleph explain` (SPEC §8, §11): a deterministic summary of a checked file.
//!
//! The report lists, in this order: the declared capabilities (always first,
//! ADR-0008), the record types, the functions in file order (canonical order,
//! `main` last), and the entry point.
//!
//! # Text format
//!
//! One item per line, without blank lines:
//!
//! ```text
//! capabilities: none
//! type point x:f64 y:f64
//! fn add a:i64 b:i64 -> i64
//!   doc: "sum"
//!   effectful: no
//!   examples: add 2 3 = 5
//!   calls: -
//!   builtins: -
//! entry: main
//! ```
//!
//! - A capability is `capabilities: CAP [ "path"]`, one line each; `none`
//!   means no capability was declared.
//! - A type is `type NAME [ field:TYPE]...` with declared field order, or
//!   for a sum `type NAME | VARIANT [ field:TYPE]...`... (ADR-0034).
//! - A function header is the canonical `fn NAME [param:TYPE]... -> TYPE`.
//! - `doc` is quoted and escaped (`"..."`; `\n`, `\t`, `\"`, `\\`) or `-`
//!   when absent, so null and the empty string stay distinguishable.
//! - `examples` repeats its prefix once per example; `-` means no examples.
//! - `calls` and `builtins` are space-separated sorted names, or `-`.
//! - `entry` is `main` or `-` when the file has no `main` function.
//! - `lambdas` (ADR-0037) appears only for a function whose body has a `lam`:
//!   one line per lambda in pre-order, `a function of x [and y] returning
//!   BODY` with the canonical body text. Without lambdas the line (and the
//!   JSON field) is omitted, so other output is unchanged.
//!
//! Everything is derived from ordered vectors or [`BTreeSet`]s, so output
//! never depends on hash-map order. ADR-0023 fixes this layout and the JSON
//! field names as a contract.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::ast::{self, CalleeKind, Expr, ExprKind, StmtKind};
use crate::{builtins, caps, examples, lexicon, printer};

/// The whole `aleph explain` result. Field order is the JSON field order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub file: String,
    pub capabilities: Vec<CapabilityInfo>,
    pub types: Vec<TypeInfo>,
    pub functions: Vec<FunctionInfo>,
    pub entry: Option<String>,
}

/// One declared capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityInfo {
    pub cap: String,
    pub path: Option<String>,
}

/// One record or sum type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    /// Sum variants (ADR-0034); omitted for a record.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<VariantInfo>,
}

/// One sum variant with its fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
}

/// One record field, with its canonical type spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// One function and its call-graph summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub signature: String,
    pub params: Vec<ParamInfo>,
    pub ret: String,
    pub doc: Option<String>,
    pub effectful: bool,
    pub examples: Vec<String>,
    pub calls: Vec<String>,
    pub builtins: Vec<String>,
    /// Lambdas of the body in pre-order (ADR-0037); omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lambdas: Vec<String>,
    /// Guards of the body in source order (ADR-0044); omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub guards: Vec<String>,
}

/// One function parameter, with its canonical type spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParamInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// Build the report for a parsed, type-clean and capability-clean file.
pub fn report(file: &str, ast: &ast::File) -> Report {
    let user_fns: BTreeSet<&str> = ast
        .fns
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    let effectful = examples::effectful_fns(ast);

    let mut types: Vec<&ast::TypeDef> = ast.types.iter().collect();
    types.sort_by(|a, b| a.name.cmp(&b.name));

    Report {
        file: file.to_string(),
        capabilities: caps::capabilities(ast)
            .into_iter()
            .map(|(cap, path)| CapabilityInfo { cap, path })
            .collect(),
        types: types
            .into_iter()
            .map(|item| TypeInfo {
                name: item.name.clone(),
                fields: field_infos(&item.fields),
                variants: item
                    .variants
                    .iter()
                    .map(|variant| VariantInfo {
                        name: variant.name.clone(),
                        fields: field_infos(&variant.fields),
                    })
                    .collect(),
            })
            .collect(),
        functions: ast
            .fns
            .iter()
            .map(|function| function_info(function, &user_fns, &effectful))
            .collect(),
        entry: ast
            .fns
            .iter()
            .any(|function| function.name == "main")
            .then(|| "main".to_string()),
    }
}

fn field_infos(fields: &[ast::Field]) -> Vec<FieldInfo> {
    fields
        .iter()
        .map(|field| FieldInfo {
            name: field.name.clone(),
            ty: printer::print_type(&field.ty),
        })
        .collect()
}

fn function_info(
    function: &ast::Fn,
    user_fns: &BTreeSet<&str>,
    effectful: &BTreeSet<String>,
) -> FunctionInfo {
    let params: Vec<ParamInfo> = function
        .params
        .iter()
        .map(|param| ParamInfo {
            name: param.name.clone(),
            ty: printer::print_type(&param.ty),
        })
        .collect();
    let ret = printer::print_type(&function.ret);

    let mut header = format!("{} {}", keyword("function"), function.name);
    for param in &params {
        header.push(' ');
        header.push_str(&param.name);
        header.push(':');
        header.push_str(&param.ty);
    }
    header.push(' ');
    header.push_str(operator("arrow"));
    header.push(' ');
    header.push_str(&ret);

    let (calls, builtins) = call_graph(function, user_fns);

    FunctionInfo {
        name: function.name.clone(),
        signature: header,
        params,
        ret,
        doc: function.doc.clone(),
        effectful: effectful.contains(&function.name),
        examples: function
            .examples
            .iter()
            .map(|example| {
                format!(
                    "{} = {}",
                    printer::print_expr(&example.lhs),
                    printer::print_expr(&example.rhs)
                )
            })
            .collect(),
        calls,
        builtins,
        lambdas: lambdas(function),
        guards: guards(function),
    }
}

/// `if C, return E` for each guard of the body, in source order (ADR-0044).
fn guards(function: &ast::Fn) -> Vec<String> {
    function
        .stmts
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StmtKind::Guard { cond, value } => Some(format!(
                "if {}, return {}",
                printer::print_expr(cond),
                printer::print_expr(value)
            )),
            _ => None,
        })
        .collect()
}

/// `a function of x and y returning BODY` for each `lam` of the body (not
/// the examples), in pre-order.
fn lambdas(function: &ast::Fn) -> Vec<String> {
    let mut found = Vec::new();
    let mut visit = |expr: &Expr| {
        if let ExprKind::Lam { binders, body } = &expr.kind {
            found.push(format!(
                "a function of {} returning {}",
                binders.join(" and "),
                printer::print_expr(body)
            ));
        }
    };
    for stmt in &function.stmts {
        match &stmt.kind {
            StmtKind::Let { value, .. } | StmtKind::Do { value } => {
                ast::walk_expr(value, &mut visit)
            }
            StmtKind::Guard { cond, value } => {
                ast::walk_expr(cond, &mut visit);
                ast::walk_expr(value, &mut visit);
            }
        }
    }
    ast::walk_expr(&function.result, &mut visit);
    found
}

/// User functions and builtins reached from the body (statements and result,
/// never examples). Calls and `@f` references both count.
fn call_graph(function: &ast::Fn, user_fns: &BTreeSet<&str>) -> (Vec<String>, Vec<String>) {
    let mut calls = BTreeSet::new();
    let mut used = BTreeSet::new();
    let mut visit = |expr: &Expr| match &expr.kind {
        ExprKind::Call {
            callee,
            callee_kind,
            ..
        } => match callee_kind {
            CalleeKind::Fn if user_fns.contains(callee.as_str()) => {
                calls.insert(callee.clone());
            }
            CalleeKind::Builtin if builtins::lookup(callee).is_some() => {
                used.insert(callee.clone());
            }
            _ => {}
        },
        ExprKind::FnRef { name } => {
            if user_fns.contains(name.as_str()) {
                calls.insert(name.clone());
            } else if builtins::lookup(name).is_some() {
                used.insert(name.clone());
            }
        }
        _ => {}
    };

    for stmt in &function.stmts {
        match &stmt.kind {
            StmtKind::Let { value, .. } | StmtKind::Do { value } => {
                ast::walk_expr(value, &mut visit)
            }
            StmtKind::Guard { cond, value } => {
                ast::walk_expr(cond, &mut visit);
                ast::walk_expr(value, &mut visit);
            }
        }
    }
    ast::walk_expr(&function.result, &mut visit);

    (calls.into_iter().collect(), used.into_iter().collect())
}

/// Render a report in the text format documented at the module level.
pub fn to_text(report: &Report) -> String {
    let mut lines = Vec::new();

    if report.capabilities.is_empty() {
        lines.push("capabilities: none".to_string());
    } else {
        for capability in &report.capabilities {
            let mut line = format!("capabilities: {}", capability.cap);
            if let Some(path) = &capability.path {
                line.push_str(" \"");
                line.push_str(&ast::escape_str(path));
                line.push('"');
            }
            lines.push(line);
        }
    }

    for item in &report.types {
        let mut line = format!("{} {}", keyword("record"), item.name);
        push_fields(&mut line, &item.fields);
        for variant in &item.variants {
            line.push(' ');
            line.push_str(operator("bar"));
            line.push(' ');
            line.push_str(&variant.name);
            push_fields(&mut line, &variant.fields);
        }
        lines.push(line);
    }

    for function in &report.functions {
        lines.push(function.signature.clone());
        let doc = match &function.doc {
            Some(doc) => format!("\"{}\"", ast::escape_str(doc)),
            None => "-".to_string(),
        };
        lines.push(format!("  doc: {doc}"));
        lines.push(format!(
            "  effectful: {}",
            if function.effectful { "yes" } else { "no" }
        ));
        if function.examples.is_empty() {
            lines.push("  examples: -".to_string());
        } else {
            for example in &function.examples {
                lines.push(format!("  examples: {example}"));
            }
        }
        lines.push(format!("  calls: {}", names(&function.calls)));
        lines.push(format!("  builtins: {}", names(&function.builtins)));
        for lambda in &function.lambdas {
            lines.push(format!("  lambdas: {lambda}"));
        }
        for guard in &function.guards {
            lines.push(format!("  guards: {guard}"));
        }
    }

    lines.push(format!("entry: {}", report.entry.as_deref().unwrap_or("-")));

    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn push_fields(line: &mut String, fields: &[FieldInfo]) {
    for field in fields {
        line.push(' ');
        line.push_str(&field.name);
        line.push(':');
        line.push_str(&field.ty);
    }
}

fn keyword(concept: &str) -> &'static str {
    lexicon::keyword(concept).expect("lexicon keyword")
}

fn operator(concept: &str) -> &'static str {
    lexicon::operator(concept).expect("lexicon operator")
}

fn names(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(" ")
    }
}

/// Render a report as pretty JSON (two-space indent) with a trailing LF.
pub fn to_json(report: &Report) -> String {
    let mut text = serde_json::to_string_pretty(report).expect("explain report serializes");
    text.push('\n');
    text
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

    #[test]
    fn capabilities_come_first_sorted_with_optional_paths() {
        let file =
            parse_ok("use fs.write \"b\"\nuse env\nuse fs.read \"a\"\nfn main -> unit\n nil\n");
        let report = report("t.aph", &file);
        assert_eq!(
            report.capabilities,
            vec![
                CapabilityInfo {
                    cap: "env".to_string(),
                    path: None,
                },
                CapabilityInfo {
                    cap: "fs.read".to_string(),
                    path: Some("a".to_string()),
                },
                CapabilityInfo {
                    cap: "fs.write".to_string(),
                    path: Some("b".to_string()),
                },
            ]
        );

        let text = to_text(&report);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "capabilities: env");
        assert_eq!(lines[1], "capabilities: fs.read \"a\"");
        assert_eq!(lines[2], "capabilities: fs.write \"b\"");
    }

    #[test]
    fn empty_capabilities_render_none_and_entry_is_null_without_main() {
        let file = parse_ok("fn helper -> i64\n 1\n");
        let report = report("t.aph", &file);
        assert!(report.capabilities.is_empty());
        assert_eq!(report.entry, None);
        let text = to_text(&report);
        assert!(text.starts_with("capabilities: none\n"));
        assert!(text.ends_with("entry: -\n"));
    }

    #[test]
    fn types_and_functions_use_canonical_type_spelling() {
        let file = parse_ok(
            "type point x:f64 y:list i64\n\
             fn main -> unit\n nil\n\
             fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n + a b\n",
        );
        let report = report("t.aph", &file);
        assert_eq!(
            report.types,
            vec![TypeInfo {
                name: "point".to_string(),
                fields: vec![
                    FieldInfo {
                        name: "x".to_string(),
                        ty: "f64".to_string(),
                    },
                    FieldInfo {
                        name: "y".to_string(),
                        ty: "list i64".to_string(),
                    },
                ],
                variants: Vec::new(),
            }]
        );
        // File order is canonical order: `main` last.
        let names: Vec<&str> = report
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, ["add", "main"]);
        assert_eq!(report.functions[0].signature, "fn add a:i64 b:i64 -> i64");
    }

    #[test]
    fn calls_and_builtins_include_fnrefs_and_are_sorted() {
        let file = parse_ok(
            "fn zed n:i64 -> i64\n + n 1\n\
             fn alpha n:i64 -> i64\n + n 1\n\
             fn useall xs:list i64 -> i64\n let ys map @zed xs\n sum filter @alpha ys\n\
             fn main -> unit\n nil\n",
        );
        let report = report("t.aph", &file);
        let function = report
            .functions
            .iter()
            .find(|function| function.name == "useall")
            .expect("useall is reported");
        assert_eq!(function.calls, ["alpha", "zed"]);
        assert_eq!(function.builtins, ["filter", "map", "sum"]);
        assert!(!function.effectful);
    }

    #[test]
    fn guards_render_and_visit_both_expressions() {
        let file = parse_ok("fn f n:i64 -> i64\n if > n 0 + n 1\n n\n");
        let report = report("t.aph", &file);
        let function = &report.functions[0];
        assert_eq!(function.guards, ["if > n 0, return + n 1"]);
        assert!(to_text(&report).contains("  guards: if > n 0, return + n 1\n"));
    }

    #[test]
    fn effectfulness_reuses_the_examples_fixpoint() {
        let file = parse_ok(
            "fn shout n:i64 -> i64\n do print show n\n n\n\
             fn relay n:i64 -> i64\n shout n\n\
             fn main -> unit\n nil\n",
        );
        let report = report("t.aph", &file);
        let effectful: Vec<&str> = report
            .functions
            .iter()
            .filter(|function| function.effectful)
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(effectful, ["relay", "shout"]);
    }

    #[test]
    fn text_marks_empty_lists_and_escapes_docs() {
        let file = parse_ok("fn f -> unit\n doc \"a\\nb\"\n nil\n");
        let report = report("t.aph", &file);
        let text = to_text(&report);
        assert!(text.contains("  doc: \"a\\nb\"\n"), "{text}");
        assert!(text.contains("  examples: -\n"), "{text}");
        assert!(text.contains("  calls: -\n"), "{text}");
        assert!(text.contains("  builtins: -\n"), "{text}");
    }

    #[test]
    fn json_field_order_and_trailing_newline_are_stable() {
        let file = parse_ok("fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n + a b\n");
        let json = to_json(&report("t.aph", &file));
        assert!(json.ends_with("}\n"));
        assert!(json.starts_with("{\n  \"file\": \"t.aph\",\n  \"capabilities\": [],"));
        assert!(json.contains(
            "  \"functions\": [\n    {\n      \"name\": \"add\",\n      \"signature\": \"fn add a:i64 b:i64 -> i64\",\n      \"params\": ["
        ));
        assert!(json.contains("\"type\": \"i64\""));
        assert!(json.contains("\"doc\": null"));
        assert!(json.contains("\"effectful\": false"));
        assert!(json.contains("\"examples\": [\n        \"add 2 3 = 5\"\n      ]"));
        assert!(json.contains("\"entry\": null"));
    }
}
