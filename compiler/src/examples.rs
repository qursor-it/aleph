//! Inline example checker and runner (SPEC §3.3, §12, ADR-0010, ADR-0022).
//!
//! `aleph check` reports `E001` for pure functions without examples and `E002`
//! for failing examples. `aleph test` runs the examples only (`E002`).
//!
//! Effectfulness is computed with a deterministic fixpoint over the body call
//! graph: a function is effectful when its statements or result reach an
//! effectful builtin (`print`, `eprint`, `read`, `write`) directly, through a
//! user function, or through a `@fnref` argument. Examples do not count
//! (ADR-0022). Examples run hermetically: filesystem builtins trap R003.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{self, CalleeKind, Expr, ExprKind, Fn, NodeId, StmtKind};
use crate::builtins::{self, Effect};
use crate::diag::Diagnostic;
use crate::interp::{self, Env, Interp};

/// `aleph check`: `E001` for missing examples, then `E002` for every failing
/// example.
pub fn check(file: &str, ast: &ast::File) -> Vec<Diagnostic> {
    let mut diagnostics = missing_examples(file, ast);
    diagnostics.extend(test(file, ast));
    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.col));
    diagnostics
}

/// `aleph test`: run every example and report `E002` failures only.
///
/// Evaluation uses the interpreter with no program arguments, an empty
/// environment, discarding stdout/stderr sinks and denied filesystem builtins
/// (`read`/`write` trap R003 without touching the disk). It runs on the same
/// large stack as `aleph run`, so a deep example traps (R004) instead of
/// crashing.
pub fn test(file: &str, ast: &ast::File) -> Vec<Diagnostic> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("aleph-examples".to_string())
            .stack_size(interp::MAIN_STACK_SIZE)
            .spawn_scoped(scope, || evaluate(file, ast))
            .expect("failed to spawn the examples thread")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    })
}

fn evaluate(file: &str, ast: &ast::File) -> Vec<Diagnostic> {
    let mut out = std::io::sink();
    let mut err = std::io::sink();
    let mut interp = Interp::new(ast, Vec::new(), &mut out, &mut err).with_deny_fs(true);
    let env = Env::new();
    let mut diagnostics = Vec::new();

    for function in &ast.fns {
        for example in &function.examples {
            let lhs = interp.eval_expr(&example.lhs, &env);
            let rhs = interp.eval_expr(&example.rhs, &env);
            let (expected, found) = match (lhs, rhs) {
                (Ok(left), Ok(right)) => {
                    if interp::values_equal(&left, &right) {
                        continue;
                    }
                    (interp::literal(&right), interp::literal(&left))
                }
                (Err(trap), Ok(right)) => (interp::literal(&right), trap_text(&trap)),
                (Ok(left), Err(trap)) => (trap_text(&trap), interp::literal(&left)),
                (Err(left_trap), Err(right_trap)) => {
                    (trap_text(&right_trap), trap_text(&left_trap))
                }
            };
            diagnostics.push(
                Diagnostic::error(
                    "E002",
                    file,
                    example.span.line,
                    example.span.col,
                    "example failed",
                )
                .with_node(node_name(example.id))
                .with_expected_found(expected, found),
            );
        }
    }

    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.col));
    diagnostics
}

fn trap_text(trap: &interp::Trap) -> String {
    format!("trap {}", trap.code)
}

fn missing_examples(file: &str, ast: &ast::File) -> Vec<Diagnostic> {
    let effectful = effectful_fns(ast);
    let mut diagnostics = Vec::new();
    for function in &ast.fns {
        if function.name == "main"
            || effectful.contains(&function.name)
            || !function.examples.is_empty()
        {
            continue;
        }
        diagnostics.push(
            Diagnostic::error(
                "E001",
                file,
                function.span.line,
                function.span.col,
                "missing examples",
            )
            .with_node(node_name(function.id)),
        );
    }
    diagnostics
}

/// The set of functions that reach an effectful builtin, directly or
/// transitively (ADR-0022).
pub(crate) fn effectful_fns(ast: &ast::File) -> BTreeSet<String> {
    let user_fns: BTreeSet<&str> = ast
        .fns
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut effectful: BTreeSet<String> = BTreeSet::new();

    for function in &ast.fns {
        let mut callees = BTreeSet::new();
        let mut directly_effectful = false;
        walk_body(function, &mut |expr| match &expr.kind {
            ExprKind::Call {
                callee,
                callee_kind: CalleeKind::Fn,
                ..
            } => {
                callees.insert(callee.clone());
            }
            ExprKind::Call {
                callee,
                callee_kind: CalleeKind::Builtin,
                ..
            } => {
                if is_effectful_builtin(callee) {
                    directly_effectful = true;
                }
            }
            ExprKind::FnRef { name } => {
                if is_effectful_builtin(name) {
                    directly_effectful = true;
                } else if user_fns.contains(name.as_str()) {
                    callees.insert(name.clone());
                }
            }
            _ => {}
        });
        if directly_effectful {
            effectful.insert(function.name.clone());
        }
        edges.insert(function.name.clone(), callees);
    }

    loop {
        let mut next = effectful.clone();
        for (caller, callees) in &edges {
            if callees.iter().any(|callee| next.contains(callee)) {
                next.insert(caller.clone());
            }
        }
        if next == effectful {
            return effectful;
        }
        effectful = next;
    }
}

fn is_effectful_builtin(name: &str) -> bool {
    builtins::lookup(name).is_some_and(|spec| spec.effect == Effect::Effectful)
}

/// Walk a function body (statements and result) but not its examples:
/// effectfulness is a property of the code, not of the tests (ADR-0022).
fn walk_body(function: &Fn, visit: &mut impl FnMut(&Expr)) {
    for stmt in &function.stmts {
        match &stmt.kind {
            StmtKind::Let { value, .. } | StmtKind::Do { value } => ast::walk_expr(value, visit),
            StmtKind::Guard { cond, value } => {
                ast::walk_expr(cond, visit);
                ast::walk_expr(value, visit);
            }
        }
    }
    ast::walk_expr(&function.result, visit);
}

fn node_name(id: NodeId) -> String {
    format!("n{}", id.0)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::interp::{FnTarget, Value};
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

    fn diagnostics(source: &str) -> Vec<Diagnostic> {
        check("t.aph", &parse_ok(source))
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    fn str_value(text: &str) -> Value {
        Value::Str(Rc::from(text))
    }

    fn list(values: Vec<Value>) -> Value {
        Value::List(Rc::new(values))
    }

    fn record(name: &str, fields: Vec<Value>) -> Value {
        Value::Record(Rc::from(name), Rc::new(fields))
    }

    #[test]
    fn effectfulness_is_direct_transitive_and_through_fnrefs() {
        let file = parse_ok(
            "fn say n:i64 -> i64\n do print \"x\"\n n\n\
             fn viasay n:i64 -> i64\n say n\n\
             fn viamap xs:list i64 -> list i64\n map @say xs\n\
             fn pure n:i64 -> i64\n + n 1\n",
        );
        let effectful = effectful_fns(&file);
        assert!(effectful.contains("say"));
        assert!(effectful.contains("viasay"));
        assert!(effectful.contains("viamap"));
        assert!(!effectful.contains("pure"));
    }

    #[test]
    fn effectfulness_ignores_examples() {
        let file = parse_ok(
            "fn noisy n:i64 -> unit\n ex noisy 1 = print \"x\"\n nil\n\
             fn caller n:i64 -> unit\n noisy n\n\
             fn main -> unit\n nil\n",
        );
        let effectful = effectful_fns(&file);
        assert!(!effectful.contains("noisy"));
        assert!(!effectful.contains("caller"));

        let diagnostics = check("t.aph", &file);
        assert_eq!(codes(&diagnostics), ["E001"]);
        assert_eq!(diagnostics[0].node.as_deref(), Some("n1"));
        assert_eq!(diagnostics[0].line, 4);
    }

    #[test]
    fn literal_text_is_deterministic_aleph_syntax() {
        assert_eq!(interp::literal(&Value::I64(-3)), "-3");
        assert_eq!(interp::literal(&Value::F64(1.0)), "1.0");
        assert_eq!(interp::literal(&Value::F64(1.5)), "1.5");
        assert_eq!(interp::literal(&Value::Bool(true)), "true");
        assert_eq!(
            interp::literal(&Value::Str(Rc::from("a\"b\n"))),
            "\"a\\\"b\\n\""
        );
        assert_eq!(interp::literal(&Value::Unit), "nil");
        assert_eq!(
            interp::literal(&list(vec![Value::I64(1), str_value("x")])),
            "[1 \"x\"]"
        );
        assert_eq!(interp::literal(&list(Vec::new())), "[]");
        assert_eq!(
            interp::literal(&Value::Opt(Some(Rc::new(Value::I64(1))))),
            "some 1"
        );
        assert_eq!(interp::literal(&Value::Opt(None)), "none");
        assert_eq!(
            interp::literal(&Value::Res(Ok(Rc::new(Value::I64(1))))),
            "ok 1"
        );
        assert_eq!(
            interp::literal(&Value::Res(Err(Rc::new(str_value("x"))))),
            "err \"x\""
        );
        assert_eq!(
            interp::literal(&record(
                "point",
                vec![Value::I64(1), list(vec![Value::Bool(false)])]
            )),
            "point 1 [false]"
        );
        assert_eq!(
            interp::literal(&Value::Fn(FnTarget::User("f".to_string()))),
            "@f"
        );
    }

    #[test]
    fn e001_reports_missing_examples_at_the_fn_header() {
        let diagnostics = diagnostics("fn twice n:i64 -> i64\n * n 2\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["E001"]);
        assert_eq!(diagnostics[0].msg, "missing examples");
        assert_eq!(diagnostics[0].line, 1);
        assert_eq!(diagnostics[0].col, 1);
        assert_eq!(diagnostics[0].node.as_deref(), Some("n1"));
    }

    #[test]
    fn effectful_functions_do_not_need_examples() {
        let diagnostics = diagnostics(
            "fn talk -> unit\n do print \"x\"\n nil\n\
             fn shout n:i64 -> unit\n do print show n\n nil\n\
             fn relay n:i64 -> unit\n shout n\n\
             fn main -> unit\n do relay 1\n nil\n",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn e002_compares_values_structurally() {
        let diagnostics =
            diagnostics("fn twice n:i64 -> i64\n ex twice 2 = 5\n * n 2\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["E002"]);
        assert_eq!(diagnostics[0].msg, "example failed");
        assert_eq!(diagnostics[0].expected.as_deref(), Some("5"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("4"));
        assert_eq!(diagnostics[0].line, 2);
        assert_eq!(diagnostics[0].col, 2);
        assert_eq!(diagnostics[0].node.as_deref(), Some("n5"));
    }

    #[test]
    fn e002_reports_a_trap_as_found() {
        let diagnostics =
            diagnostics("fn div n:i64 -> i64\n ex div 0 = 1\n / 1 n\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["E002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("1"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("trap R002"));
    }

    #[test]
    fn e002_reports_rhs_only_trap_with_the_lhs_value() {
        let diagnostics =
            diagnostics("fn value -> i64\n ex value = / 1 0\n 1\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["E002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("trap R002"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("1"));
    }

    #[test]
    fn e002_reports_both_traps_with_lhs_found_rhs_expected() {
        let diagnostics = diagnostics(
            "fn over n:i64 -> i64\n ex over 9223372036854775807 = / 1 0\n + n 1\n\
             fn main -> unit\n nil\n",
        );
        assert_eq!(codes(&diagnostics), ["E002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("trap R002"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("trap R001"));
    }

    #[test]
    fn examples_trap_r003_for_filesystem_builtins() {
        let diagnostics = diagnostics(
            "use fs.write \"data\"\n\
             fn save -> res unit str\n ex save = write \"data/out.txt\" \"x\"\n ok nil\n\
             fn main -> unit\n nil\n",
        );
        assert_eq!(codes(&diagnostics), ["E002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("trap R003"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("ok nil"));
    }

    #[test]
    fn all_examples_run_even_when_one_fails() {
        let diagnostics = diagnostics(
            "fn same n:i64 -> i64\n ex same 1 = 9\n ex same 2 = 9\n n\nfn main -> unit\n nil\n",
        );
        assert_eq!(codes(&diagnostics), ["E002", "E002"]);
    }

    #[test]
    fn lists_options_results_records_and_floats_compare_structurally() {
        let diagnostics = diagnostics(
            "type point x:i64 y:i64\n\
             fn make -> point\n ex make = point 1 2\n point 1 2\n\
             fn pick o:opt i64 -> opt i64\n ex pick some 1 = some 1\n ex pick none i64 = none i64\n o\n\
             fn done -> res i64 str\n ex done = ok 7\n ok 7\n\
             fn items -> list i64\n ex items = [1 2 3]\n [1 2 3]\n\
             fn ratio -> f64\n ex ratio = 1.5\n 1.5\n\
             fn main -> unit\n nil\n",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn float_mismatch_uses_show_spelling() {
        let diagnostics =
            diagnostics("fn ratio -> f64\n ex ratio = 1.5\n 1.0\nfn main -> unit\n nil\n");
        assert_eq!(codes(&diagnostics), ["E002"]);
        assert_eq!(diagnostics[0].expected.as_deref(), Some("1.5"));
        assert_eq!(diagnostics[0].found.as_deref(), Some("1.0"));
    }

    #[test]
    fn try_at_example_top_level_is_the_err_value() {
        let clean = diagnostics(
            "fn fails -> res i64 str\n ex fails = err \"x\"\n err \"x\"\nfn main -> unit\n nil\n",
        );
        assert!(clean.is_empty(), "{clean:?}");

        let failed = diagnostics(
            "fn fails -> res i64 str\n ex fails = err \"y\"\n err \"x\"\nfn main -> unit\n nil\n",
        );
        assert_eq!(codes(&failed), ["E002"]);
        assert_eq!(failed[0].expected.as_deref(), Some("err \"y\""));
        assert_eq!(failed[0].found.as_deref(), Some("err \"x\""));
    }

    #[test]
    fn test_runs_examples_without_requiring_them() {
        let file = parse_ok("fn identity n:i64 -> i64\n n\nfn main -> unit\n nil\n");
        assert!(test("t.aph", &file).is_empty());

        let file =
            parse_ok("fn twice n:i64 -> i64\n ex twice 2 = 5\n * n 2\nfn main -> unit\n nil\n");
        let diagnostics = test("t.aph", &file);
        assert_eq!(codes(&diagnostics), ["E002"]);
    }
}
