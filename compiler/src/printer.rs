//! Canonical printer (SPEC §1-3, §10, ADR-0003).
//!
//! The AST is the source of truth: [`print_file`] renders it as the single
//! canonical text. Every keyword and operator spelling comes from [`lexicon`]
//! or the AST spelling helpers; only punctuation that the lexicon does not
//! model (`:`, `[`, `]`, `=`, `"`) is written literally.

use crate::ast::{self, BinOp, ExprKind, StmtKind, TypeKind, UnOp};
use crate::lexicon;

/// Print an AST as canonical Aleph source.
///
/// Items keep their [`ast::File`] order (the parser already canonicalizes via
/// [`ast::File::finish`]). The canonical form of an empty module is the empty
/// string; every non-empty file ends with exactly one LF.
///
/// # Precondition
///
/// `file` must be parser-producible: list expressions are non-empty, float
/// literals use text the lexer accepts, `Named` types are non-builtin
/// identifiers, and so on. The printer does not validate the AST.
pub fn print_file(file: &ast::File) -> String {
    let mut lines = Vec::new();
    lines.extend(file.uses.iter().map(ast::Use::canonical_text));
    lines.extend(file.types.iter().map(print_typedef));
    for function in &file.fns {
        lines.extend(print_fn(function));
    }

    let mut text = lines.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

fn keyword(concept: &str) -> &'static str {
    lexicon::keyword(concept).expect("lexicon keyword")
}

fn type_name(concept: &str) -> &'static str {
    lexicon::type_name(concept).expect("lexicon type name")
}

fn punctuation(concept: &str) -> &'static str {
    lexicon::operator(concept).expect("lexicon punctuation")
}

fn quoted(value: &str) -> String {
    let mut text = String::with_capacity(value.len() + 2);
    text.push('"');
    text.push_str(&ast::escape_str(value));
    text.push('"');
    text
}

pub(crate) fn print_typedef(item: &ast::TypeDef) -> String {
    let mut text = format!("{} {}", keyword("record"), item.name);
    print_fields(&mut text, &item.fields);
    for variant in &item.variants {
        text.push(' ');
        text.push_str(punctuation("bar"));
        text.push(' ');
        text.push_str(&variant.name);
        print_fields(&mut text, &variant.fields);
    }
    text
}

fn print_fields(text: &mut String, fields: &[ast::Field]) {
    for field in fields {
        text.push(' ');
        text.push_str(&field.name);
        text.push(':');
        text.push_str(&print_type(&field.ty));
    }
}

pub(crate) fn print_fn(function: &ast::Fn) -> Vec<String> {
    let mut header = format!("{} {}", keyword("function"), function.name);
    for param in &function.params {
        header.push(' ');
        header.push_str(&param.name);
        header.push(':');
        header.push_str(&print_type(&param.ty));
    }
    header.push(' ');
    header.push_str(punctuation("arrow"));
    header.push(' ');
    header.push_str(&print_type(&function.ret));

    let mut lines = vec![header];
    if let Some(doc) = &function.doc {
        lines.push(format!("{} {}", keyword("intent"), quoted(doc)));
    }
    for example in &function.examples {
        lines.push(format!(
            "{} {} = {}",
            keyword("example"),
            print_expr(&example.lhs),
            print_expr(&example.rhs)
        ));
    }
    for stmt in &function.stmts {
        let line = match &stmt.kind {
            StmtKind::Let { name, value } => {
                format!("{} {} {}", keyword("bind"), name, print_expr(value))
            }
            StmtKind::Do { value } => format!("{} {}", keyword("effect"), print_expr(value)),
        };
        lines.push(line);
    }
    lines.push(print_expr(&function.result));

    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| if index == 0 { line } else { format!(" {line}") })
        .collect()
}

/// Canonical type spelling, shared with `aleph explain`.
pub(crate) fn print_type(ty: &ast::Type) -> String {
    match &ty.kind {
        TypeKind::I64 => type_name("int").to_string(),
        TypeKind::F64 => type_name("float").to_string(),
        TypeKind::Bool => type_name("boolean").to_string(),
        TypeKind::Str => type_name("string").to_string(),
        TypeKind::Unit => type_name("unit").to_string(),
        TypeKind::Json => type_name("json").to_string(),
        TypeKind::List { elem } => format!("{} {}", type_name("list"), print_type(elem)),
        TypeKind::Opt { elem } => format!("{} {}", type_name("option"), print_type(elem)),
        TypeKind::Res { ok, err } => format!(
            "{} {} {}",
            type_name("result"),
            print_type(ok),
            print_type(err)
        ),
        TypeKind::Dict { key, val } => {
            format!(
                "{} {} {}",
                type_name("dict"),
                print_type(key),
                print_type(val)
            )
        }
        TypeKind::Named { name } => name.clone(),
    }
}

fn print_binop(op: BinOp) -> &'static str {
    op.spelling()
}

fn print_unop(op: UnOp) -> &'static str {
    op.spelling()
}

/// Canonical expression text, shared with `aleph explain`.
pub fn print_expr(expr: &ast::Expr) -> String {
    match &expr.kind {
        ExprKind::Int { value } => value.to_string(),
        ExprKind::Float { value } => value.clone(),
        ExprKind::Str { value } => quoted(value),
        ExprKind::Bool { value } => keyword(if *value { "true" } else { "false" }).to_string(),
        ExprKind::Nil => keyword("unitval").to_string(),
        ExprKind::Var { name } => name.clone(),
        ExprKind::FnRef { name } => format!("{}{}", punctuation("fnref"), name),
        ExprKind::Call { callee, args, .. } => {
            let mut text = callee.clone();
            for arg in args {
                text.push(' ');
                text.push_str(&print_expr(arg));
            }
            text
        }
        ExprKind::BinOp { op, lhs, rhs } => format!(
            "{} {} {}",
            print_binop(*op),
            print_expr(lhs),
            print_expr(rhs)
        ),
        ExprKind::UnOp { op, operand } => {
            format!("{} {}", print_unop(*op), print_expr(operand))
        }
        ExprKind::If { cond, then_, else_ } => format!(
            "{} {} {} {}",
            keyword("cond"),
            print_expr(cond),
            print_expr(then_),
            print_expr(else_)
        ),
        ExprKind::List { elems } => {
            let printed: Vec<String> = elems.iter().map(print_expr).collect();
            format!("[{}]", printed.join(" "))
        }
        ExprKind::EmptyList => "[]".to_string(),
        ExprKind::Empty { ty } => format!("{} {}", keyword("emptylist"), print_type(ty)),
        ExprKind::Dict { ty } => print_type(ty),
        ExprKind::Field { name, value } => {
            format!("{}{} {}", punctuation("field"), name, print_expr(value))
        }
        ExprKind::Some { value } => format!("{} {}", keyword("optsome"), print_expr(value)),
        ExprKind::None { ty } => format!("{} {}", keyword("optnone"), print_type(ty)),
        ExprKind::Ok { value } => format!("{} {}", keyword("resok"), print_expr(value)),
        ExprKind::Err { value } => format!("{} {}", keyword("reserr"), print_expr(value)),
        ExprKind::Try { value } => format!("{} {}", keyword("propagate"), print_expr(value)),
        ExprKind::Else { value, default } => format!(
            "{} {} {}",
            keyword("default"),
            print_expr(value),
            print_expr(default)
        ),
        ExprKind::Has { value } => format!("{} {}", keyword("isome"), print_expr(value)),
        ExprKind::IsOk { value } => format!("{} {}", keyword("isokay"), print_expr(value)),
        ExprKind::Match { value, arms } => {
            let mut text = format!("{} {}", keyword("match"), print_expr(value));
            for arm in arms {
                text.push(' ');
                text.push_str(&arm.variant);
                for binder in &arm.binders {
                    text.push(' ');
                    text.push_str(binder);
                }
                text.push(' ');
                text.push_str(&print_expr(&arm.body));
            }
            text
        }
        ExprKind::Lam { binders, body } => {
            let mut text = keyword("lambda").to_string();
            for binder in binders {
                text.push(' ');
                text.push_str(binder);
            }
            text.push(' ');
            text.push_str(&print_expr(body));
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::ast::{
        BinOp, CalleeKind, Capability, Example, File, Fn, NodeId, Param, Span, Stmt, TypeDef, Use,
    };
    use crate::builtins;
    use crate::parser;

    fn parse_ok(source: &str) -> File {
        let output = parser::parse("t.aph", source);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}\nsource:\n{source}",
            output.diagnostics
        );
        output.file.expect("valid source has an AST")
    }

    fn print(source: &str) -> String {
        print_file(&parse_ok(source))
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("compiler/ has a parent")
            .to_path_buf()
    }

    #[test]
    fn print_parse_round_trips_canonical_snippets() {
        let snippets = [
            "fn f x:i64 -> i64\n x\n",
            "fn f -> i64\n 42\n",
            "fn f -> i64\n -7\n",
            "fn f -> f64\n -1.50\n",
            "fn f -> str\n \"a\\nb\\t\\\"c\\\\\"\n",
            "fn f -> bool\n true\n",
            "fn f -> bool\n false\n",
            "fn f -> unit\n nil\n",
            "fn add a:i64 b:i64 -> i64\n + a b\n",
            "fn f b:bool -> bool\n not b\n",
            "fn f x:i64 -> i64\n neg x\n",
            "fn f b:bool -> i64\n if b 1 2\n",
            "fn f -> list i64\n [1 2 3]\n",
            "fn f -> list list i64\n [[1 2] [3]]\n",
            "fn f -> list i64\n empty i64\n",
            "fn f -> opt i64\n none i64\n",
            "fn f -> opt i64\n some 1\n",
            "fn f -> res i64 str\n ok 1\n",
            "fn f -> res i64 str\n err \"x\"\n",
            "fn f -> res i64 str\n try ok 1\n",
            "fn f o:opt i64 -> i64\n else o 0\n",
            "fn f o:opt i64 -> bool\n has o\n",
            "fn f r:res i64 str -> bool\n isok r\n",
            "fn f p:point -> f64\n .x p\n",
            "fn f xs:list i64 -> i64\n len xs\n",
            "fn add a:i64 b:i64 -> i64\n + a b\nfn f -> i64\n @add\n",
            "fn add a:i64 b:i64 -> i64\n doc \"sum\"\n ex add 2 3 = 5\n let s + a b\n s\n",
            "fn main -> unit\n do print \"hi\"\n nil\n",
            "use fs.read \"data\"\ntype point x:f64 y:f64\nfn main -> unit\n nil\n",
            "type shape | circle r:f64 | rect w:f64 h:f64 | dot\nfn area s:shape -> f64\n match s circle r * r r rect w h * w h dot 0.0\n",
            "type t | a x:list t | b\nfn f v:t -> i64\n + 1 match v a xs len xs b 0\n",
            "fn f xs:list i64 k:i64 -> i64\n fold lam a x + a * k x 0 filter lam x > x k map lam y map2 lam p q + p q [y] xs xs\n",
        ];

        for source in snippets {
            assert_eq!(print(source), source, "round trip failed for {source:?}");
        }
    }

    #[test]
    fn print_parse_round_trips_spec_examples() {
        let examples = [
            r#"fn add a:i64 b:i64 -> i64
 ex add 2 3 = 5
 + a b
fn fib n:i64 -> i64
 doc "nth fibonacci number"
 ex fib 0 = 0
 ex fib 10 = 55
 if < n 2 n + fib - n 1 fib - n 2
fn total xs:list i64 -> i64
 ex total [1 2 3] = 6
 ex total empty i64 = 0
 fold @add 0 xs
fn main -> unit
 print show fib 20
"#,
            r#"use fs.read "data"
fn words s:str -> i64
 ex words "a b c" = 3
 len split trim s " "
fn main -> res unit str
 let text try read "data/in.txt"
 do print show words text
 ok nil
"#,
        ];

        for source in examples {
            assert_eq!(print(source), source);
        }
    }

    #[test]
    fn print_parse_round_trips_golden_programs() {
        let directory = repo_root().join("tests/aph");
        let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
            .expect("tests/aph exists")
            .map(|entry| entry.expect("readable dir entry").path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
            .filter(|path| {
                path.with_extension("out").is_file() || path.with_extension("ast").is_file()
            })
            .collect();
        cases.sort();
        assert!(!cases.is_empty(), "no golden programs in {directory:?}");

        for path in cases {
            let source = fs::read_to_string(&path).expect("read golden program");
            assert_eq!(
                print(&source),
                source,
                "round trip failed for {}",
                path.display()
            );
        }
    }

    #[test]
    fn non_canonical_input_is_printed_canonically() {
        let mangled = "fn  add  a:i64  b:i64 -> i64   \n\n ex add 2 3 = 5   \n + a b\n";
        let canonical = "fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n + a b\n";
        assert_eq!(print(mangled), canonical);

        let mangled = "fn add a:i64 b:i64 -> i64\n let s + a b\n ex add 2 3 = 5\n s\n";
        let canonical = "fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n let s + a b\n s\n";
        assert_eq!(print(mangled), canonical);

        let mangled = "fn main -> unit\n print \"hi\"\nuse env\ntype point x:f64 y:f64\n";
        let canonical = "use env\ntype point x:f64 y:f64\nfn main -> unit\n print \"hi\"\n";
        assert_eq!(print(mangled), canonical);
    }

    #[test]
    fn empty_file_prints_empty_text() {
        assert_eq!(print(""), "");
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }
    }

    fn span() -> Span {
        Span { line: 1, col: 1 }
    }

    fn ty(kind: TypeKind) -> ast::Type {
        ast::Type {
            id: NodeId::UNASSIGNED,
            span: span(),
            kind,
        }
    }

    fn expr(kind: ExprKind) -> ast::Expr {
        ast::Expr {
            id: NodeId::UNASSIGNED,
            span: span(),
            kind,
        }
    }

    fn gen_type(rng: &mut Rng, depth: usize) -> ast::Type {
        if depth == 0 {
            return ty(match rng.below(6) {
                0 => TypeKind::I64,
                1 => TypeKind::F64,
                2 => TypeKind::Bool,
                3 => TypeKind::Str,
                4 => TypeKind::Unit,
                _ => TypeKind::Named {
                    name: "point".to_string(),
                },
            });
        }
        match rng.below(9) {
            0 => ty(TypeKind::I64),
            1 => ty(TypeKind::F64),
            2 => ty(TypeKind::Bool),
            3 => ty(TypeKind::Str),
            4 => ty(TypeKind::Unit),
            5 => ty(TypeKind::List {
                elem: Box::new(gen_type(rng, depth - 1)),
            }),
            6 => ty(TypeKind::Opt {
                elem: Box::new(gen_type(rng, depth - 1)),
            }),
            7 => ty(TypeKind::Res {
                ok: Box::new(gen_type(rng, depth - 1)),
                err: Box::new(gen_type(rng, depth - 1)),
            }),
            _ => ty(TypeKind::Named {
                name: "point".to_string(),
            }),
        }
    }

    fn gen_leaf(rng: &mut Rng, fns: &[(String, usize)], env: &[String]) -> ast::Expr {
        let has_var = !env.is_empty();
        let choices = if has_var { 7 } else { 6 };
        match rng.below(choices) {
            0 => {
                let ints = [0, 1, -1, 42, -100, i64::MIN, i64::MAX];
                expr(ExprKind::Int {
                    value: ints[rng.below(ints.len() as u64) as usize],
                })
            }
            1 => {
                let floats = ["0.0", "-1.5", "0.5", "-0.25", "10.0", "3.14", "1.00"];
                expr(ExprKind::Float {
                    value: floats[rng.below(floats.len() as u64) as usize].to_string(),
                })
            }
            2 => expr(ExprKind::Str {
                value: ["", "a b", "line\nbreak", "quote \" and \\"][rng.below(4) as usize]
                    .to_string(),
            }),
            3 => expr(ExprKind::Bool {
                value: rng.below(2) == 0,
            }),
            4 => expr(ExprKind::Nil),
            5 => expr(ExprKind::FnRef {
                name: fns[rng.below(fns.len() as u64) as usize].0.clone(),
            }),
            _ => expr(ExprKind::Var {
                name: env[rng.below(env.len() as u64) as usize].clone(),
            }),
        }
    }

    fn random_binop(rng: &mut Rng) -> BinOp {
        [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::Div,
            BinOp::Mod,
            BinOp::Eq,
            BinOp::Ne,
            BinOp::Lt,
            BinOp::Le,
            BinOp::Gt,
            BinOp::Ge,
            BinOp::And,
            BinOp::Or,
        ][rng.below(13) as usize]
    }

    fn gen_expr(rng: &mut Rng, fns: &[(String, usize)], env: &[String], depth: usize) -> ast::Expr {
        if depth == 0 {
            return gen_leaf(rng, fns, env);
        }
        match rng.below(25) {
            0..=6 => gen_leaf(rng, fns, env),
            7 => expr(ExprKind::UnOp {
                op: if rng.below(2) == 0 {
                    UnOp::Not
                } else {
                    UnOp::Neg
                },
                operand: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            8 => expr(ExprKind::BinOp {
                op: random_binop(rng),
                lhs: Box::new(gen_expr(rng, fns, env, depth - 1)),
                rhs: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            9 => expr(ExprKind::If {
                cond: Box::new(gen_expr(rng, fns, env, depth - 1)),
                then_: Box::new(gen_expr(rng, fns, env, depth - 1)),
                else_: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            10 => {
                let count = rng.below(3) + 1;
                expr(ExprKind::List {
                    elems: (0..count)
                        .map(|_| gen_expr(rng, fns, env, depth - 1))
                        .collect(),
                })
            }
            11 => expr(ExprKind::Empty {
                ty: gen_type(rng, depth - 1),
            }),
            12 => expr(ExprKind::None {
                ty: gen_type(rng, depth - 1),
            }),
            13 => expr(ExprKind::Field {
                name: ["x", "y"][rng.below(2) as usize].to_string(),
                value: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            14 => expr(ExprKind::Some {
                value: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            15 => expr(if rng.below(2) == 0 {
                ExprKind::Ok {
                    value: Box::new(gen_expr(rng, fns, env, depth - 1)),
                }
            } else {
                ExprKind::Err {
                    value: Box::new(gen_expr(rng, fns, env, depth - 1)),
                }
            }),
            16 => expr(ExprKind::Try {
                value: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            17 => expr(ExprKind::Else {
                value: Box::new(gen_expr(rng, fns, env, depth - 1)),
                default: Box::new(gen_expr(rng, fns, env, depth - 1)),
            }),
            18 => expr(if rng.below(2) == 0 {
                ExprKind::Has {
                    value: Box::new(gen_expr(rng, fns, env, depth - 1)),
                }
            } else {
                ExprKind::IsOk {
                    value: Box::new(gen_expr(rng, fns, env, depth - 1)),
                }
            }),
            19 => {
                let (name, arity) = &fns[rng.below(fns.len() as u64) as usize];
                expr(ExprKind::Call {
                    callee: name.clone(),
                    callee_kind: CalleeKind::Fn,
                    args: (0..*arity)
                        .map(|_| gen_expr(rng, fns, env, depth - 1))
                        .collect(),
                })
            }
            20 => expr(ExprKind::Call {
                callee: "point".to_string(),
                callee_kind: CalleeKind::Ctor,
                args: (0..2).map(|_| gen_expr(rng, fns, env, depth - 1)).collect(),
            }),
            21 => {
                let (callee, arity) = [("circle", 1), ("dot", 0)][rng.below(2) as usize];
                expr(ExprKind::Call {
                    callee: callee.to_string(),
                    callee_kind: CalleeKind::Ctor,
                    args: (0..arity)
                        .map(|_| gen_expr(rng, fns, env, depth - 1))
                        .collect(),
                })
            }
            22 => {
                // Binder names are unique along any path, so they never shadow (T005).
                let binder = format!("m{depth}");
                let mut inner = env.to_vec();
                inner.push(binder.clone());
                expr(ExprKind::Match {
                    value: Box::new(gen_expr(rng, fns, env, depth - 1)),
                    arms: vec![
                        ast::Arm {
                            variant: "circle".to_string(),
                            binders: vec![binder],
                            body: gen_expr(rng, fns, &inner, depth - 1),
                        },
                        ast::Arm {
                            variant: "dot".to_string(),
                            binders: Vec::new(),
                            body: gen_expr(rng, fns, env, depth - 1),
                        },
                    ],
                })
            }
            23 => {
                let name = ["map", "filter", "fold", "map2"][rng.below(4) as usize];
                let spec = builtins::lookup(name).expect("higher-order builtin");
                expr(ExprKind::Call {
                    callee: name.to_string(),
                    callee_kind: CalleeKind::Builtin,
                    args: spec
                        .params
                        .iter()
                        .map(|param| match param {
                            builtins::Ty::Fn(params, _) => {
                                gen_lam(rng, fns, env, depth, params.len())
                            }
                            _ => gen_expr(rng, fns, env, depth - 1),
                        })
                        .collect(),
                })
            }
            _ => {
                let spec = &builtins::BUILTINS[rng.below(builtins::BUILTINS.len() as u64) as usize];
                expr(ExprKind::Call {
                    callee: spec.name.to_string(),
                    callee_kind: CalleeKind::Builtin,
                    args: spec
                        .params
                        .iter()
                        .map(|param| match param {
                            builtins::Ty::Fn(params, _) if rng.below(2) == 0 => {
                                gen_lam(rng, fns, env, depth, params.len())
                            }
                            _ => gen_expr(rng, fns, env, depth - 1),
                        })
                        .collect(),
                })
            }
        }
    }

    /// `lam` with `count` binders; names carry `depth`, so they are unique
    /// along any path and never shadow (T005).
    fn gen_lam(
        rng: &mut Rng,
        fns: &[(String, usize)],
        env: &[String],
        depth: usize,
        count: usize,
    ) -> ast::Expr {
        let binders: Vec<String> = ["l", "k"][..count]
            .iter()
            .map(|prefix| format!("{prefix}{depth}"))
            .collect();
        let mut inner = env.to_vec();
        inner.extend(binders.iter().cloned());
        expr(ExprKind::Lam {
            binders,
            body: Box::new(gen_expr(rng, fns, &inner, depth - 1)),
        })
    }

    fn gen_file(rng: &mut Rng) -> File {
        let signatures = [
            ("f0".to_string(), 0usize),
            ("f1".to_string(), 1usize),
            ("f2".to_string(), 2usize),
        ];

        let mut fns = Vec::new();
        for (name, arity) in &signatures {
            let params: Vec<Param> = (0..*arity)
                .map(|index| Param {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    name: format!("p{index}"),
                    ty: gen_type(rng, 2),
                })
                .collect();
            let ret = gen_type(rng, 2);
            let mut env: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

            let examples = (0..rng.below(3))
                .map(|_| Example {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    lhs: gen_expr(rng, &signatures, &[], 4),
                    rhs: gen_expr(rng, &signatures, &[], 4),
                })
                .collect();

            let mut stmts = Vec::new();
            for index in 0..rng.below(3) {
                let binding = format!("v{index}");
                let value = gen_expr(rng, &signatures, &env, 4);
                env.push(binding.clone());
                stmts.push(Stmt {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    kind: StmtKind::Let {
                        name: binding,
                        value,
                    },
                });
            }
            if rng.below(3) == 0 {
                stmts.push(Stmt {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    kind: StmtKind::Do {
                        value: gen_expr(rng, &signatures, &env, 4),
                    },
                });
            }
            let result = gen_expr(rng, &signatures, &env, 5);
            let docs = [
                "generated function",
                "quote \" backslash \\ newline \n tab \t end",
                "",
            ];
            let doc = if rng.below(3) == 0 {
                Some(docs[rng.below(docs.len() as u64) as usize].to_string())
            } else {
                None
            };

            fns.push(Fn {
                id: NodeId::UNASSIGNED,
                span: span(),
                name: name.clone(),
                params,
                ret,
                doc,
                examples,
                stmts,
                result,
            });
        }

        // Include every capability; randomize whether it has a path and which
        // escapes the path exercises.
        let paths = ["data", "a\"b", "a\\b", "a\nb", "a\tb", ""];
        let capabilities = [
            Capability::FsRead,
            Capability::FsWrite,
            Capability::NetHost,
            Capability::Env,
            Capability::Time,
        ];
        let uses = capabilities
            .into_iter()
            .map(|cap| Use {
                id: NodeId::UNASSIGNED,
                span: span(),
                cap,
                path: if rng.below(2) == 0 {
                    None
                } else {
                    Some(paths[rng.below(paths.len() as u64) as usize].to_string())
                },
            })
            .collect();

        let types = vec![
            TypeDef {
                id: NodeId::UNASSIGNED,
                span: span(),
                name: "point".to_string(),
                fields: vec!["x", "y"]
                    .into_iter()
                    .map(|name| ast::Field {
                        id: NodeId::UNASSIGNED,
                        span: span(),
                        name: name.to_string(),
                        ty: ty(TypeKind::F64),
                    })
                    .collect(),
                variants: Vec::new(),
            },
            TypeDef {
                id: NodeId::UNASSIGNED,
                span: span(),
                name: "shape".to_string(),
                fields: Vec::new(),
                variants: vec![("circle", true), ("dot", false)]
                    .into_iter()
                    .map(|(name, has_field)| ast::Variant {
                        id: NodeId::UNASSIGNED,
                        span: span(),
                        name: name.to_string(),
                        fields: if has_field {
                            vec![ast::Field {
                                id: NodeId::UNASSIGNED,
                                span: span(),
                                name: "r".to_string(),
                                ty: ty(TypeKind::F64),
                            }]
                        } else {
                            Vec::new()
                        },
                    })
                    .collect(),
            },
        ];

        File {
            id: NodeId::UNASSIGNED,
            span: span(),
            uses,
            types,
            fns,
        }
    }

    #[test]
    fn parse_print_round_trips_generated_asts() {
        let mut matches = 0;
        let mut lams = 0;
        for case in 0..500u64 {
            let mut rng = Rng(0x5eed_0000 ^ case);
            let mut file = gen_file(&mut rng);
            file.finish();

            let text = print_file(&file);
            matches += text.matches(" match ").count();
            lams += text.matches(" lam ").count();
            let output = parser::parse("gen.aph", &text);
            assert!(
                output.diagnostics.is_empty(),
                "case {case}: generated text did not parse: {:?}\n{text}",
                output.diagnostics
            );
            let reparsed = output.file.expect("valid generated text has an AST");
            assert!(
                file.same_ids(&reparsed),
                "case {case}: AST round trip failed\n{text}\ngenerated: {file:#?}\nreparsed: {reparsed:#?}"
            );
        }
        assert!(
            matches > 100,
            "only {matches} generated `match` expressions"
        );
        assert!(lams > 100, "only {lams} generated `lam` expressions");
    }
}
