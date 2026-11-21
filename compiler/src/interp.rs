//! Interpreter for checked Aleph programs (SPEC §5, §5.1, §6, ADR-0009).
//!
//! The interpreter assumes a parsed, type- and capability-clean
//! [`ast::File`]: expression shapes and builtin arities are trusted. Evaluation
//! is strict and left-to-right; `and`/`or`/`if` only evaluate the branches they
//! need. Runtime failures are [`Trap`]s with codes `R001`–`R005`.
//!
//! Values are immutable and shared with [`Rc`] where that is cheap. Lists have
//! value semantics: every list builtin returns a new list.
//!
//! `try` is the only early exit. Internally expressions evaluate to a
//! [`Flow`], where `Return` unwinds to the enclosing user function; `call_fn`
//! turns it back into that function's value.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

use crate::ast::{
    self, BinOp, CalleeKind, Capability, Expr, ExprKind, NodeId, Span, StmtKind, UnOp,
};
use crate::builtins;
use crate::caps;
use crate::diag::Diagnostic;
use crate::json;
use crate::lexicon;

/// The maximum number of nested user function calls (R004).
pub const MAX_CALL_DEPTH: usize = 10_000;

/// The maximum total evaluation depth (R004). Measured so a 250-deep
/// expression body recursing at its innermost node traps cleanly without
/// exhausting the 256 MiB thread stack (see
/// `deep_nested_recursion_traps_without_sigsegv`), while 5 000 simple
/// recursive calls still succeed.
pub const MAX_EVAL_DEPTH: usize = 20_000;

/// The largest list `range` may allocate (R005).
pub const MAX_RANGE_LEN: i64 = 10_000_000;
/// The longest string `repeat` may build, in bytes (R005, ADR-0042).
pub const MAX_REPEAT_BYTES: usize = 100_000_000;

/// Stack size for [`run_main`]'s interpreter thread.
pub const MAIN_STACK_SIZE: usize = 256 * 1024 * 1024;

/// A callable value: a user function, a record constructor, a builtin, or a
/// lambda.
#[derive(Debug, Clone)]
pub enum FnTarget {
    User(String),
    Ctor(String),
    Builtin(&'static str),
    /// A `lam` node and the values of the outer names its body reads,
    /// captured when the `lam` was evaluated (ADR-0037).
    Lam(NodeId, Rc<Env>),
}

impl PartialEq for FnTarget {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::User(a), Self::User(b)) | (Self::Ctor(a), Self::Ctor(b)) => a == b,
            (Self::Builtin(a), Self::Builtin(b)) => a == b,
            (Self::Lam(a, a_env), Self::Lam(b, b_env)) => a == b && Rc::ptr_eq(a_env, b_env),
            _ => false,
        }
    }
}

/// A runtime value (SPEC §6).
#[derive(Debug, Clone)]
pub enum Value {
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(Rc<str>),
    Unit,
    List(Rc<Vec<Value>>),
    Opt(Option<Rc<Value>>),
    Res(Result<Rc<Value>, Rc<Value>>),
    Record(Rc<str>, Rc<Vec<Value>>),
    /// Key/value pairs sorted by key with [`compare_values`] (ADR-0032).
    Dict(Rc<Vec<(Value, Value)>>),
    /// A JSON value: its payload and nesting depth (ADR-0033, see
    /// [`crate::json`]).
    Json(Rc<Value>, u32),
    Fn(FnTarget),
}

/// Structural equality used by examples and `==`/`!=` (ADR-0021).
///
/// Floats compare with IEEE `==`, so `NaN != NaN`. The traversal uses an
/// explicit stack so a deeply nested value does not overflow the Rust stack.
pub fn values_equal(left: &Value, right: &Value) -> bool {
    let mut stack: Vec<(&Value, &Value)> = vec![(left, right)];
    while let Some((left, right)) = stack.pop() {
        match (left, right) {
            (Value::I64(a), Value::I64(b)) if a != b => return false,
            (Value::F64(a), Value::F64(b)) if a != b => return false,
            (Value::Bool(a), Value::Bool(b)) if a != b => return false,
            (Value::Str(a), Value::Str(b)) if a != b => return false,
            (Value::I64(_), Value::I64(_))
            | (Value::F64(_), Value::F64(_))
            | (Value::Bool(_), Value::Bool(_))
            | (Value::Str(_), Value::Str(_))
            | (Value::Unit, Value::Unit) => {}
            (Value::List(a), Value::List(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                stack.extend(a.iter().zip(b.iter()));
            }
            (Value::Opt(a), Value::Opt(b)) => match (a, b) {
                (None, None) => {}
                (Some(a), Some(b)) => stack.push((a, b)),
                _ => return false,
            },
            (Value::Res(a), Value::Res(b)) => match (a, b) {
                (Ok(a), Ok(b)) | (Err(a), Err(b)) => stack.push((a, b)),
                _ => return false,
            },
            (Value::Record(an, af), Value::Record(bn, bf)) => {
                if an != bn || af.len() != bf.len() {
                    return false;
                }
                stack.extend(af.iter().zip(bf.iter()));
            }
            (Value::Dict(a), Value::Dict(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for ((ak, av), (bk, bv)) in a.iter().zip(b.iter()) {
                    stack.push((ak, bk));
                    stack.push((av, bv));
                }
            }
            // Equal when `show` prints the same text, so `-0.0` differs from `0.0`.
            (Value::Json(a, _), Value::Json(b, _)) => {
                if crate::json::print(a) != crate::json::print(b) {
                    return false;
                }
            }
            (Value::Fn(a), Value::Fn(b)) if a != b => return false,
            (Value::Fn(_), Value::Fn(_)) => {}
            _ => return false,
        }
    }
    true
}

/// Move every uniquely-owned child of `value` into `stack`.
///
/// Shared children are left alone: dropping their `Rc` only decrements a
/// count, so no recursive destruction happens there.
fn take_children(value: &mut Value, stack: &mut Vec<Value>) {
    match value {
        Value::List(items) => {
            if let Some(items) = Rc::get_mut(items) {
                stack.append(items);
            }
        }
        Value::Opt(Some(inner)) => {
            if let Some(inner) = Rc::get_mut(inner) {
                stack.push(std::mem::replace(inner, Value::Unit));
            }
        }
        Value::Res(Ok(inner)) | Value::Res(Err(inner)) => {
            if let Some(inner) = Rc::get_mut(inner) {
                stack.push(std::mem::replace(inner, Value::Unit));
            }
        }
        Value::Record(_, fields) => {
            if let Some(fields) = Rc::get_mut(fields) {
                stack.append(fields);
            }
        }
        Value::Dict(pairs) => {
            if let Some(pairs) = Rc::get_mut(pairs) {
                for (key, val) in pairs.drain(..) {
                    stack.push(key);
                    stack.push(val);
                }
            }
        }
        Value::Json(payload, _) => {
            if let Some(payload) = Rc::get_mut(payload) {
                stack.push(std::mem::replace(payload, Value::Unit));
            }
        }
        Value::Fn(FnTarget::Lam(_, captured)) => {
            if let Some(captured) = Rc::get_mut(captured) {
                for value in captured.values_mut() {
                    stack.push(std::mem::replace(value, Value::Unit));
                }
            }
        }
        _ => {}
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        let mut stack = Vec::new();
        take_children(self, &mut stack);
        while let Some(mut value) = stack.pop() {
            take_children(&mut value, &mut stack);
        }
    }
}

/// A structured runtime trap (SPEC §6, §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trap {
    /// `R001`, `R002`, `R003` or `R004`.
    pub code: &'static str,
    pub msg: String,
    pub node: NodeId,
    pub span: Span,
}

impl Trap {
    fn new(code: &'static str, msg: &str, node: NodeId, span: Span) -> Self {
        Self {
            code,
            msg: msg.to_string(),
            node,
            span,
        }
    }

    /// Render this trap as a diagnostic (`node` is the failing AST node).
    pub fn to_diagnostic(&self, file: &str) -> Diagnostic {
        Diagnostic::error(
            self.code,
            file,
            self.span.line,
            self.span.col,
            self.msg.clone(),
        )
        .with_node(format!("n{}", self.node.0))
    }
}

/// The variable environment of one function body.
pub type Env = BTreeMap<String, Value>;

/// The names visible to an expression: a function body's `Env`, or binders
/// (match or lambda) layered on an outer scope without copying it (T-052).
#[derive(Clone, Copy)]
enum Scope<'e> {
    Root(&'e Env),
    Bind {
        names: &'e [String],
        values: &'e [Value],
        outer: &'e Scope<'e>,
    },
}

impl<'e> Scope<'e> {
    fn get(&self, name: &str) -> Option<&'e Value> {
        match *self {
            Scope::Root(env) => env.get(name),
            Scope::Bind {
                names,
                values,
                outer,
            } => names
                .iter()
                .zip(values)
                .rev()
                .find(|(binder, _)| *binder == name)
                .map(|(_, value)| value)
                .or_else(|| outer.get(name)),
        }
    }
}

/// A `lam` node and the distinct names its body reads, which are the
/// candidates for capture (ADR-0037).
struct LamInfo<'a> {
    expr: &'a Expr,
    reads: Vec<&'a str>,
}

/// Internal control flow: a value, or an early return produced by `try`.
enum Flow {
    Value(Value),
    Return(Value),
}

/// Evaluate a sub-expression, propagating a `try` early return.
macro_rules! eval_value {
    ($interp:expr, $expr:expr, $env:expr) => {
        match $interp.eval($expr, $env)? {
            Flow::Value(value) => value,
            Flow::Return(value) => return Ok(Flow::Return(value)),
        }
    };
}

/// The interpreter state.
pub struct Interp<'a> {
    file: &'a ast::File,
    args: Vec<String>,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    fns: BTreeMap<&'a str, &'a ast::Fn>,
    /// Constructor (record type or sum variant) name to its fields.
    records: BTreeMap<&'a str, &'a [ast::Field]>,
    /// Every `lam` node of the file, by id (ADR-0037).
    lams: BTreeMap<NodeId, LamInfo<'a>>,
    call_depth: usize,
    eval_depth: usize,
    /// When set, every `read`/`write`/`env`/`now`/`fetch` call traps with R003
    /// without touching the filesystem, environment, clock or network (used for
    /// hermetic inline examples, ADR-0022).
    deny_fs: bool,
}

impl<'a> Interp<'a> {
    /// Build an interpreter over a checked file.
    pub fn new(
        file: &'a ast::File,
        args: Vec<String>,
        out: &'a mut dyn Write,
        err: &'a mut dyn Write,
    ) -> Self {
        let fns = file
            .fns
            .iter()
            .map(|function| (function.name.as_str(), function))
            .collect();
        let mut records = BTreeMap::new();
        for item in &file.types {
            if item.variants.is_empty() {
                records.insert(item.name.as_str(), item.fields.as_slice());
            }
            for variant in &item.variants {
                records.insert(variant.name.as_str(), variant.fields.as_slice());
            }
        }
        let mut lams = BTreeMap::new();
        let mut visit = |expr: &'a Expr| {
            if let ExprKind::Lam { body, .. } = &expr.kind {
                let mut reads = BTreeSet::new();
                ast::walk_expr(body, &mut |node: &'a Expr| {
                    if let ExprKind::Var { name } = &node.kind {
                        reads.insert(name.as_str());
                    }
                });
                let reads = reads.into_iter().collect();
                lams.insert(expr.id, LamInfo { expr, reads });
            }
        };
        for function in &file.fns {
            let roots = function
                .examples
                .iter()
                .flat_map(|example| [&example.lhs, &example.rhs])
                .chain(function.stmts.iter().flat_map(|stmt| match &stmt.kind {
                    StmtKind::Let { value, .. } | StmtKind::Do { value } => vec![value],
                    StmtKind::Guard { cond, value } => vec![cond, value],
                }))
                .chain([&function.result]);
            for root in roots {
                ast::walk_expr(root, &mut visit);
            }
        }
        Self {
            file,
            args,
            out,
            err,
            fns,
            records,
            lams,
            call_depth: 0,
            eval_depth: 0,
            deny_fs: false,
        }
    }

    /// Deny the outside world: `read`, `write`, `env`, `now` and `fetch` trap
    /// with R003 without resolving paths, touching the filesystem, reading the
    /// environment or clock, or using the network (ADR-0022).
    pub fn with_deny_fs(mut self, deny_fs: bool) -> Self {
        self.deny_fs = deny_fs;
        self
    }

    /// Call a user function with `args`. `main` is just another function here.
    pub fn call_fn(&mut self, name: &str, args: Vec<Value>) -> Result<Value, Trap> {
        let function = *self
            .fns
            .get(name)
            .unwrap_or_else(|| panic!("interpreted function `{name}` does not exist"));
        self.call_user(function, args, function.id, function.span)
    }

    /// Evaluate an expression in `env`.
    ///
    /// A `try` that returns early has no enclosing function here, so its
    /// payload is returned as the value.
    pub fn eval_expr(&mut self, expr: &Expr, env: &Env) -> Result<Value, Trap> {
        match self.eval(expr, &Scope::Root(env))? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
        }
    }

    /// Execute `main` and return the process exit status (0 or 1).
    fn interpret_main(&mut self) -> Result<i32, Trap> {
        let function = *self.fns.get("main").expect("checked file has a main");
        let value = self.call_user(function, Vec::new(), function.id, function.span)?;
        match &value {
            Value::Res(Err(error)) => {
                let message = match error.as_ref() {
                    Value::Str(text) => text.to_string(),
                    other => show(other),
                };
                let _ = self.out.flush();
                let _ = self.err.write_all(message.as_bytes());
                let _ = self.err.write_all(b"\n");
                let _ = self.err.flush();
                Ok(1)
            }
            _ => Ok(0),
        }
    }

    fn eval(&mut self, expr: &Expr, env: &Scope) -> Result<Flow, Trap> {
        if self.eval_depth >= MAX_EVAL_DEPTH {
            return Err(Trap::new("R004", "stack overflow", expr.id, expr.span));
        }
        self.eval_depth += 1;
        let result = self.eval_inner(expr, env);
        self.eval_depth -= 1;
        result
    }

    fn eval_inner(&mut self, expr: &Expr, env: &Scope) -> Result<Flow, Trap> {
        match &expr.kind {
            ExprKind::Int { value } => Ok(Flow::Value(Value::I64(*value))),
            ExprKind::Float { value } => {
                let number = value.parse::<f64>().expect("lexer validates float syntax");
                Ok(Flow::Value(Value::F64(number)))
            }
            ExprKind::Str { value } => Ok(Flow::Value(Value::Str(Rc::from(value.as_str())))),
            ExprKind::Bool { value } => Ok(Flow::Value(Value::Bool(*value))),
            ExprKind::Nil => Ok(Flow::Value(Value::Unit)),
            ExprKind::Var { name } => Ok(Flow::Value(
                env.get(name)
                    .unwrap_or_else(|| panic!("unbound variable `{name}`"))
                    .clone(),
            )),
            ExprKind::FnRef { name } => Ok(Flow::Value(Value::Fn(self.fn_target(name)))),
            ExprKind::Call {
                callee,
                callee_kind,
                args,
            } => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(eval_value!(self, arg, env));
                }
                let value = match callee_kind {
                    CalleeKind::Fn => {
                        let function = *self.fns.get(callee.as_str()).unwrap_or_else(|| {
                            panic!("interpreted function `{callee}` does not exist")
                        });
                        self.call_user(function, values, expr.id, expr.span)?
                    }
                    CalleeKind::Ctor => self.construct(callee, values),
                    CalleeKind::Builtin => self.builtin(callee, values, expr.id, expr.span)?,
                };
                Ok(Flow::Value(value))
            }
            ExprKind::BinOp { op, lhs, rhs } => self.eval_binop(*op, lhs, rhs, env, expr),
            ExprKind::UnOp { op, operand } => {
                let value = eval_value!(self, operand, env);
                let value = match op {
                    UnOp::Not => Value::Bool(!as_bool(&value)),
                    UnOp::Neg => match value {
                        Value::I64(number) => {
                            Value::I64(number.checked_neg().ok_or_else(|| {
                                Trap::new("R001", "integer overflow", expr.id, expr.span)
                            })?)
                        }
                        Value::F64(number) => Value::F64(-number),
                        _ => panic!("type checker guarantees a numeric operand"),
                    },
                };
                Ok(Flow::Value(value))
            }
            ExprKind::If { cond, then_, else_ } => {
                if as_bool(&eval_value!(self, cond, env)) {
                    self.eval(then_, env)
                } else {
                    self.eval(else_, env)
                }
            }
            ExprKind::List { elems } => {
                let mut values = Vec::with_capacity(elems.len());
                for elem in elems {
                    values.push(eval_value!(self, elem, env));
                }
                Ok(Flow::Value(Value::List(Rc::new(values))))
            }
            ExprKind::Empty { .. } | ExprKind::EmptyList => {
                Ok(Flow::Value(Value::List(Rc::new(Vec::new()))))
            }
            ExprKind::Dict { .. } => Ok(Flow::Value(Value::Dict(Rc::new(Vec::new())))),
            ExprKind::Field { name, value } => {
                let record = eval_value!(self, value, env);
                let Value::Record(type_name, fields) = &record else {
                    panic!("type checker guarantees a record");
                };
                let item = self
                    .records
                    .get(type_name.as_ref())
                    .unwrap_or_else(|| panic!("unknown record `{type_name}`"));
                let index = item
                    .iter()
                    .position(|field| &field.name == name)
                    .unwrap_or_else(|| panic!("unknown field `{name}`"));
                Ok(Flow::Value(fields[index].clone()))
            }
            ExprKind::Some { value } => Ok(Flow::Value(Value::Opt(Some(Rc::new(eval_value!(
                self, value, env
            )))))),
            ExprKind::None { .. } => Ok(Flow::Value(Value::Opt(None))),
            ExprKind::Ok { value } => Ok(Flow::Value(Value::Res(Ok(Rc::new(eval_value!(
                self, value, env
            )))))),
            ExprKind::Err { value } => Ok(Flow::Value(Value::Res(Err(Rc::new(eval_value!(
                self, value, env
            )))))),
            ExprKind::Try { value } => {
                let value = eval_value!(self, value, env);
                match &value {
                    Value::Res(Ok(value)) => Ok(Flow::Value(value.as_ref().clone())),
                    Value::Res(Err(error)) => Ok(Flow::Return(Value::Res(Err(error.clone())))),
                    _ => panic!("type checker guarantees a result for try"),
                }
            }
            ExprKind::Else { value, default } => {
                let value = eval_value!(self, value, env);
                match &value {
                    Value::Opt(Some(value)) | Value::Res(Ok(value)) => {
                        Ok(Flow::Value(value.as_ref().clone()))
                    }
                    Value::Opt(None) | Value::Res(Err(_)) => self.eval(default, env),
                    _ => panic!("type checker guarantees opt or res for else"),
                }
            }
            ExprKind::Has { value } => Ok(Flow::Value(Value::Bool(matches!(
                eval_value!(self, value, env),
                Value::Opt(Some(_))
            )))),
            ExprKind::IsOk { value } => Ok(Flow::Value(Value::Bool(matches!(
                eval_value!(self, value, env),
                Value::Res(Ok(_))
            )))),
            ExprKind::Match { value, arms } => {
                let value = eval_value!(self, value, env);
                let Value::Record(variant, fields) = &value else {
                    panic!("type checker guarantees a sum value for match");
                };
                let arm = arms
                    .iter()
                    .find(|arm| arm.variant.as_str() == variant.as_ref())
                    .unwrap_or_else(|| panic!("no arm for variant `{variant}`"));
                if arm.binders.is_empty() {
                    return self.eval(&arm.body, env);
                }
                let inner = Scope::Bind {
                    names: &arm.binders,
                    values: fields.as_slice(),
                    outer: env,
                };
                self.eval(&arm.body, &inner)
            }
            ExprKind::Lam { .. } => {
                // Capture by value only the outer names the body reads.
                let mut captured = Env::new();
                for name in &self.lams[&expr.id].reads {
                    if let Some(value) = env.get(name) {
                        captured.insert(name.to_string(), value.clone());
                    }
                }
                Ok(Flow::Value(Value::Fn(FnTarget::Lam(
                    expr.id,
                    Rc::new(captured),
                ))))
            }
        }
    }

    fn eval_binop(
        &mut self,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        env: &Scope,
        expr: &Expr,
    ) -> Result<Flow, Trap> {
        if matches!(op, BinOp::And | BinOp::Or) {
            let left = as_bool(&eval_value!(self, lhs, env));
            let short_circuit = match op {
                BinOp::And => !left,
                BinOp::Or => left,
                _ => unreachable!(),
            };
            if short_circuit {
                return Ok(Flow::Value(Value::Bool(left)));
            }
            return Ok(Flow::Value(Value::Bool(as_bool(&eval_value!(
                self, rhs, env
            )))));
        }

        let left = eval_value!(self, lhs, env);
        let right = eval_value!(self, rhs, env);
        let value = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.arithmetic(op, left, right, expr.id, expr.span)?
            }
            BinOp::Eq => Value::Bool(values_equal(&left, &right)),
            BinOp::Ne => Value::Bool(!values_equal(&left, &right)),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                Value::Bool(compare(op, &left, &right))
            }
            BinOp::And | BinOp::Or => unreachable!(),
        };
        Ok(Flow::Value(value))
    }

    fn arithmetic(
        &self,
        op: BinOp,
        left: Value,
        right: Value,
        node: NodeId,
        span: Span,
    ) -> Result<Value, Trap> {
        match (left, right) {
            (Value::I64(a), Value::I64(b)) => {
                let result = match op {
                    BinOp::Add => a.checked_add(b),
                    BinOp::Sub => a.checked_sub(b),
                    BinOp::Mul => a.checked_mul(b),
                    BinOp::Div => {
                        if b == 0 {
                            return Err(Trap::new("R002", "division by zero", node, span));
                        }
                        a.checked_div(b)
                    }
                    BinOp::Mod => {
                        if b == 0 {
                            return Err(Trap::new("R002", "division by zero", node, span));
                        }
                        a.checked_rem(b)
                    }
                    _ => unreachable!(),
                };
                result
                    .map(Value::I64)
                    .ok_or_else(|| Trap::new("R001", "integer overflow", node, span))
            }
            (Value::F64(a), Value::F64(b)) => {
                let result = match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                    _ => panic!("`%` is only defined on i64"),
                };
                Ok(Value::F64(result))
            }
            _ => panic!("type checker guarantees matching numeric operands"),
        }
    }

    fn call_user(
        &mut self,
        function: &'a ast::Fn,
        args: Vec<Value>,
        node: NodeId,
        span: Span,
    ) -> Result<Value, Trap> {
        if self.call_depth >= MAX_CALL_DEPTH || self.eval_depth >= MAX_EVAL_DEPTH {
            return Err(Trap::new("R004", "stack overflow", node, span));
        }
        self.call_depth += 1;
        self.eval_depth += 1;
        let result = self.call_user_inner(function, args);
        self.eval_depth -= 1;
        self.call_depth -= 1;
        result
    }

    fn call_user_inner(&mut self, function: &'a ast::Fn, args: Vec<Value>) -> Result<Value, Trap> {
        debug_assert_eq!(
            function.params.len(),
            args.len(),
            "arity of `{}`",
            function.name
        );
        let mut env: Env = BTreeMap::new();
        for (param, value) in function.params.iter().zip(args) {
            env.insert(param.name.clone(), value);
        }

        for stmt in &function.stmts {
            match &stmt.kind {
                StmtKind::Let { name, value } => match self.eval(value, &Scope::Root(&env))? {
                    Flow::Value(value) => {
                        env.insert(name.clone(), value);
                    }
                    Flow::Return(value) => return Ok(value),
                },
                StmtKind::Do { value } => match self.eval(value, &Scope::Root(&env))? {
                    Flow::Value(_) => {}
                    Flow::Return(value) => return Ok(value),
                },
                StmtKind::Guard { cond, value } => match self.eval(cond, &Scope::Root(&env))? {
                    Flow::Return(value) => return Ok(value),
                    Flow::Value(Value::Bool(true)) => {
                        match self.eval(value, &Scope::Root(&env))? {
                            Flow::Value(value) | Flow::Return(value) => return Ok(value),
                        }
                    }
                    Flow::Value(Value::Bool(false)) => {}
                    Flow::Value(_) => panic!("type checker guarantees a bool guard condition"),
                },
            }
        }

        match self.eval(&function.result, &Scope::Root(&env))? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
        }
    }

    fn fn_target(&self, name: &str) -> FnTarget {
        if self.fns.contains_key(name) {
            FnTarget::User(name.to_string())
        } else if self.records.contains_key(name) {
            FnTarget::Ctor(name.to_string())
        } else {
            let spec =
                builtins::lookup(name).unwrap_or_else(|| panic!("`{name}` is not a callable"));
            FnTarget::Builtin(spec.name)
        }
    }

    fn call_target(
        &mut self,
        target: &FnTarget,
        args: Vec<Value>,
        node: NodeId,
        span: Span,
    ) -> Result<Value, Trap> {
        match target {
            FnTarget::User(name) => {
                let function = *self
                    .fns
                    .get(name.as_str())
                    .unwrap_or_else(|| panic!("unknown function `{name}`"));
                self.call_user(function, args, node, span)
            }
            FnTarget::Ctor(name) => Ok(self.construct(name, args)),
            FnTarget::Builtin(name) => self.builtin(name, args, node, span),
            FnTarget::Lam(id, captured) => self.call_lam(*id, captured, args, node, span),
        }
    }

    fn call_lam(
        &mut self,
        id: NodeId,
        captured: &Env,
        args: Vec<Value>,
        node: NodeId,
        span: Span,
    ) -> Result<Value, Trap> {
        let lam: &'a Expr = self.lams[&id].expr;
        let ExprKind::Lam { binders, body } = &lam.kind else {
            panic!("`n{}` is not a lambda", id.0);
        };
        if self.call_depth >= MAX_CALL_DEPTH || self.eval_depth >= MAX_EVAL_DEPTH {
            return Err(Trap::new("R004", "stack overflow", node, span));
        }
        let outer = Scope::Root(captured);
        let env = Scope::Bind {
            names: binders,
            values: &args,
            outer: &outer,
        };
        self.call_depth += 1;
        let result = self.eval(body, &env);
        self.call_depth -= 1;
        // The checker rejects `try` in a lambda body, so there is no early return.
        match result? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
        }
    }

    fn construct(&self, name: &str, args: Vec<Value>) -> Value {
        let item = self
            .records
            .get(name)
            .unwrap_or_else(|| panic!("unknown record `{name}`"));
        debug_assert_eq!(item.len(), args.len(), "arity of record `{name}`");
        Value::Record(Rc::from(name), Rc::new(args))
    }

    fn builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        node: NodeId,
        span: Span,
    ) -> Result<Value, Trap> {
        let spec = builtins::lookup(name).unwrap_or_else(|| panic!("unknown builtin `{name}`"));
        debug_assert_eq!(spec.arity(), args.len(), "arity of builtin `{name}`");
        let value = match name {
            "len" => Value::I64(as_list(&args[0]).len() as i64),
            "put" => {
                let mut pairs = as_dict(&args[0]).as_ref().clone();
                match find_key(&pairs, &args[1]) {
                    Ok(index) => pairs[index].1 = args[2].clone(),
                    Err(index) => pairs.insert(index, (args[1].clone(), args[2].clone())),
                }
                Value::Dict(Rc::new(pairs))
            }
            "get" => {
                let pairs = as_dict(&args[0]);
                match find_key(pairs, &args[1]) {
                    Ok(index) => Value::Opt(Some(Rc::new(pairs[index].1.clone()))),
                    Err(_) => Value::Opt(None),
                }
            }
            "del" => {
                let pairs = as_dict(&args[0]);
                match find_key(pairs, &args[1]) {
                    Ok(index) => {
                        let mut pairs = pairs.as_ref().clone();
                        pairs.remove(index);
                        Value::Dict(Rc::new(pairs))
                    }
                    Err(_) => args[0].clone(),
                }
            }
            "keys" => Value::List(Rc::new(
                as_dict(&args[0])
                    .iter()
                    .map(|(key, _)| key.clone())
                    .collect(),
            )),
            "vals" => Value::List(Rc::new(
                as_dict(&args[0])
                    .iter()
                    .map(|(_, val)| val.clone())
                    .collect(),
            )),
            "size" => Value::I64(as_dict(&args[0]).len() as i64),
            "at" => {
                let values = as_list(&args[0]);
                match usize::try_from(as_i64(&args[1])) {
                    Ok(index) if index < values.len() => {
                        Value::Opt(Some(Rc::new(values[index].clone())))
                    }
                    _ => Value::Opt(None),
                }
            }
            "index" => {
                let values = as_list(&args[0]);
                let target = &args[1];
                let found = values.iter().position(|value| values_equal(value, target));
                Value::Opt(found.map(|index| Rc::new(Value::I64(index as i64))))
            }
            "push" => {
                let mut values = as_list(&args[0]).as_ref().clone();
                values.push(args[1].clone());
                Value::List(Rc::new(values))
            }
            "slice" => {
                let values = as_list(&args[0]);
                let start = clamp_index(as_i64(&args[1]), values.len());
                let end = clamp_index(as_i64(&args[2]), values.len());
                if start >= end {
                    Value::List(Rc::new(Vec::new()))
                } else {
                    Value::List(Rc::new(values[start..end].to_vec()))
                }
            }
            "rev" => {
                let mut values = as_list(&args[0]).as_ref().clone();
                values.reverse();
                Value::List(Rc::new(values))
            }
            "sort" => {
                let mut values = as_list(&args[0]).as_ref().clone();
                values.sort_by(compare_values);
                Value::List(Rc::new(values))
            }
            "sortby" => {
                let target = as_fn(&args[0]);
                let values = as_list(&args[1]);
                let mut keyed = Vec::with_capacity(values.len());
                for value in values.iter() {
                    let key = self.call_target(target, vec![value.clone()], node, span)?;
                    keyed.push((key, value.clone()));
                }
                // Stable: equal keys keep their input order.
                keyed.sort_by(|left, right| compare_values(&left.0, &right.0));
                Value::List(Rc::new(keyed.into_iter().map(|(_, value)| value).collect()))
            }
            "repeat" => {
                let text = as_str(&args[0]);
                let count = usize::try_from(as_i64(&args[1])).unwrap_or(0);
                if count == 0 || text.is_empty() {
                    Value::Str(Rc::from(""))
                } else {
                    let length = text
                        .len()
                        .checked_mul(count)
                        .filter(|length| *length <= MAX_REPEAT_BYTES)
                        .ok_or_else(|| {
                            Trap::new("R005", "allocation limit exceeded", node, span)
                        })?;
                    let mut repeated = String::with_capacity(length);
                    for _ in 0..count {
                        repeated.push_str(text);
                    }
                    Value::Str(Rc::from(repeated.as_str()))
                }
            }
            "range" => {
                let start = as_i64(&args[0]);
                let end = as_i64(&args[1]);
                let values: Vec<Value> = if start >= end {
                    Vec::new()
                } else {
                    match end.checked_sub(start) {
                        Some(count) if count <= MAX_RANGE_LEN => {
                            (start..end).map(Value::I64).collect()
                        }
                        _ => {
                            return Err(Trap::new("R005", "allocation limit exceeded", node, span))
                        }
                    }
                };
                Value::List(Rc::new(values))
            }
            "sum" => {
                let mut total: i64 = 0;
                for value in as_list(&args[0]).iter() {
                    total = total
                        .checked_add(as_i64(value))
                        .ok_or_else(|| Trap::new("R001", "integer overflow", node, span))?;
                }
                Value::I64(total)
            }
            "map" => {
                let target = as_fn(&args[0]);
                let mut mapped = Vec::with_capacity(as_list(&args[1]).len());
                for value in as_list(&args[1]).iter() {
                    mapped.push(self.call_target(target, vec![value.clone()], node, span)?);
                }
                Value::List(Rc::new(mapped))
            }
            "filter" => {
                let target = as_fn(&args[0]);
                let mut kept = Vec::new();
                for value in as_list(&args[1]).iter() {
                    if as_bool(&self.call_target(target, vec![value.clone()], node, span)?) {
                        kept.push(value.clone());
                    }
                }
                Value::List(Rc::new(kept))
            }
            "fold" => {
                let target = as_fn(&args[0]);
                let mut accumulator = args[1].clone();
                for value in as_list(&args[2]).iter() {
                    accumulator =
                        self.call_target(target, vec![accumulator, value.clone()], node, span)?;
                }
                accumulator
            }
            "cat" => match &args[0] {
                Value::List(left) => {
                    let right = as_list(&args[1]);
                    let mut values = Vec::with_capacity(left.len() + right.len());
                    values.extend(left.iter().cloned());
                    values.extend(right.iter().cloned());
                    Value::List(Rc::new(values))
                }
                _ => {
                    let mut text = as_str(&args[0]).to_string();
                    text.push_str(as_str(&args[1]));
                    Value::Str(Rc::from(text.as_str()))
                }
            },
            "count" => Value::I64(as_str(&args[0]).len() as i64),
            "split" => {
                let text = as_str(&args[0]);
                let separator = as_str(&args[1]);
                let parts: Vec<Value> = if separator.is_empty() {
                    text.chars()
                        .map(|ch| Value::Str(Rc::from(ch.to_string().as_str())))
                        .collect()
                } else {
                    text.split(separator)
                        .map(|part| Value::Str(Rc::from(part)))
                        .collect()
                };
                Value::List(Rc::new(parts))
            }
            "join" => {
                let parts: Vec<&str> = as_list(&args[0]).iter().map(as_str).collect();
                Value::Str(Rc::from(parts.join(as_str(&args[1])).as_str()))
            }
            "trim" => Value::Str(Rc::from(as_str(&args[0]).trim())),
            "upper" => Value::Str(Rc::from(as_str(&args[0]).to_ascii_uppercase().as_str())),
            "lower" => Value::Str(Rc::from(as_str(&args[0]).to_ascii_lowercase().as_str())),
            "contains" => Value::Bool(as_str(&args[0]).contains(as_str(&args[1]))),
            "show" => Value::Str(Rc::from(show(&args[0]).as_str())),
            "json" => match json::wrap(args[0].clone()) {
                Some(value) => value,
                None => return Err(Trap::new("R005", "allocation limit exceeded", node, span)),
            },
            "jparse" => match json::parse(as_str(&args[0])) {
                Ok(value) => result_ok(value),
                Err(message) => result_err(&message),
            },
            "jbool" => json_field(&args[0], |payload| matches!(payload, Value::Bool(_))),
            "jint" => json_field(&args[0], |payload| matches!(payload, Value::I64(_))),
            "jnum" => match json::inner(&args[0]) {
                Value::I64(number) => Value::Opt(Some(Rc::new(Value::F64(*number as f64)))),
                payload @ Value::F64(_) => Value::Opt(Some(Rc::new(payload.clone()))),
                _ => Value::Opt(None),
            },
            "jstr" => json_field(&args[0], |payload| matches!(payload, Value::Str(_))),
            "jlist" => json_field(&args[0], |payload| matches!(payload, Value::List(_))),
            "jdict" => json_field(&args[0], |payload| matches!(payload, Value::Dict(_))),
            "int" => match parse_int(as_str(&args[0])) {
                Some(number) => result_ok(Value::I64(number)),
                None => result_err("invalid int"),
            },
            "float" => match parse_float(as_str(&args[0])) {
                Some(number) => result_ok(Value::F64(number)),
                None => result_err("invalid float"),
            },
            "args" => Value::List(Rc::new(
                self.args
                    .iter()
                    .map(|arg| Value::Str(Rc::from(arg.as_str())))
                    .collect(),
            )),
            "env" => {
                if self.deny_fs {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                let name = as_str(&args[0]);
                if name.is_empty() || name.contains('=') || name.contains('\0') {
                    return Ok(Value::Opt(None));
                }
                match std::env::var(name) {
                    Ok(text) => Value::Opt(Some(Rc::new(Value::Str(Rc::from(text.as_str()))))),
                    Err(_) => Value::Opt(None),
                }
            }
            "now" => {
                if self.deny_fs {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                // Milliseconds since the epoch, flooring nanoseconds by 1e6
                // (so times before 1970 round down, consistently with Tlön).
                let millis = match std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                {
                    Ok(elapsed) => (elapsed.as_nanos() / 1_000_000).min(i64::MAX as u128) as i64,
                    Err(error) => {
                        let before = error.duration().as_nanos().div_ceil(1_000_000);
                        -(before.min(i64::MAX as u128) as i64)
                    }
                };
                Value::I64(millis)
            }
            "print" => {
                let text = as_str(&args[0]);
                let _ = self.out.write_all(text.as_bytes());
                let _ = self.out.write_all(b"\n");
                Value::Unit
            }
            "eprint" => {
                let text = as_str(&args[0]);
                let _ = self.out.flush();
                let _ = self.err.write_all(text.as_bytes());
                let _ = self.err.write_all(b"\n");
                let _ = self.err.flush();
                Value::Unit
            }
            "fetch" => {
                if self.deny_fs {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                let url = as_str(&args[0]);
                if !self.host_allowed(url) {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                fetch(url)
            }
            "read" => {
                if self.deny_fs {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                let path = as_str(&args[0]);
                if !self.path_allowed(Capability::FsRead, path) {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                match fs::read_to_string(path) {
                    Ok(text) => result_ok(Value::Str(Rc::from(text.as_str()))),
                    Err(error) => result_err(&io_error_text(&error)),
                }
            }
            "write" => {
                if self.deny_fs {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                let path = as_str(&args[0]);
                if !self.path_allowed(Capability::FsWrite, path) {
                    return Err(Trap::new("R003", "capability violation", node, span));
                }
                match fs::write(path, as_str(&args[1]).as_bytes()) {
                    Ok(()) => result_ok(Value::Unit),
                    Err(error) => result_err(&io_error_text(&error)),
                }
            }
            "abs" => match &args[0] {
                Value::I64(number) => Value::I64(
                    number
                        .checked_abs()
                        .ok_or_else(|| Trap::new("R001", "integer overflow", node, span))?,
                ),
                Value::F64(number) => Value::F64(number.abs()),
                _ => panic!("type checker guarantees a numeric argument"),
            },
            "sqrt" => match &args[0] {
                Value::F64(number) => Value::F64(number.sqrt()),
                _ => panic!("type checker guarantees an f64 argument"),
            },
            "tof" => match &args[0] {
                Value::I64(number) => Value::F64(*number as f64),
                _ => panic!("type checker guarantees an i64 argument"),
            },
            "min" | "max" => {
                let take_right = match (&args[0], &args[1]) {
                    (Value::I64(left), Value::I64(right)) => {
                        if name == "min" {
                            right < left
                        } else {
                            right > left
                        }
                    }
                    (Value::F64(left), Value::F64(right)) => {
                        if name == "min" {
                            right < left
                        } else {
                            right > left
                        }
                    }
                    _ => panic!("type checker guarantees matching numeric arguments"),
                };
                if take_right {
                    args[1].clone()
                } else {
                    args[0].clone()
                }
            }
            "ord" => Value::Opt(
                as_str(&args[0])
                    .chars()
                    .next()
                    .map(|c| Rc::new(Value::I64(i64::from(u32::from(c))))),
            ),
            "chr" => Value::Opt(
                u32::try_from(as_i64(&args[0]))
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| Rc::new(Value::Str(Rc::from(c.to_string())))),
            ),
            "starts" => Value::Bool(as_str(&args[0]).starts_with(as_str(&args[1]))),
            "ends" => Value::Bool(as_str(&args[0]).ends_with(as_str(&args[1]))),
            "uniq" => {
                let mut values = as_list(&args[0]).as_ref().clone();
                values.sort_by(compare_values);
                values.dedup_by(|right, left| compare_values(left, right).is_eq());
                Value::List(Rc::new(values))
            }
            "map2" => {
                let target = as_fn(&args[0]);
                let left = as_list(&args[1]);
                let right = as_list(&args[2]);
                let mut mapped = Vec::with_capacity(left.len().min(right.len()));
                for (a, b) in left.iter().zip(right.iter()) {
                    mapped.push(self.call_target(
                        target,
                        vec![a.clone(), b.clone()],
                        node,
                        span,
                    )?);
                }
                Value::List(Rc::new(mapped))
            }
            "mapr" => {
                let target = as_fn(&args[0]);
                let values = as_list(&args[1]);
                let mut mapped = Vec::with_capacity(values.len());
                for value in values.iter() {
                    let item = self.call_target(target, vec![value.clone()], node, span)?;
                    if matches!(&item, Value::Res(Err(_))) {
                        return Ok(item);
                    }
                    match &item {
                        Value::Res(Ok(inner)) => mapped.push(inner.as_ref().clone()),
                        _ => panic!("type checker guarantees `mapr` returns a result"),
                    }
                }
                result_ok(Value::List(Rc::new(mapped)))
            }
            _ => panic!("builtin `{name}` has no interpreter implementation"),
        };
        Ok(value)
    }

    /// Whether the literal URL's host is declared by a `net.host` capability.
    fn host_allowed(&self, url: &str) -> bool {
        let Some(host) = caps::url_host(url) else {
            return false;
        };
        self.file.uses.iter().any(|item| {
            item.cap == Capability::NetHost
                && item
                    .path
                    .as_deref()
                    .is_some_and(|declared| declared.eq_ignore_ascii_case(host))
        })
    }

    /// Whether `path` is covered by a declared prefix for `capability`.
    ///
    /// Both sides are resolved with [`resolve_lenient`] to absolute paths and
    /// compared component-wise with [`Path::starts_with`] (ADR-0021). This
    /// resolves symlinks in existing ancestors, rejects `..`, and treats a
    /// dangling final symlink as a capability violation rather than silently
    /// creating its target.
    fn path_allowed(&self, capability: Capability, path: &str) -> bool {
        let Some(target) = resolve_lenient(path) else {
            return false;
        };
        self.file.uses.iter().any(|item| {
            if item.cap != capability {
                return false;
            }
            let Some(prefix) = &item.path else {
                return false;
            };
            let Some(prefix) = resolve_lenient(prefix) else {
                return false;
            };
            target.starts_with(&prefix)
        })
    }
}

/// Execute `main` on a thread with [`MAIN_STACK_SIZE`] bytes of stack,
/// streaming to the process stdout and stderr.
///
/// Buffers are flushed before returning, so the caller can write a trap to
/// stderr without interleaving it with pending program output. Returns the
/// exit status: 0 for `unit`/`ok`, 1 for `err` (whose payload is written to
/// stderr followed by a newline). A trap is returned to the caller.
pub fn run_main(file: &ast::File, args: Vec<String>) -> Result<i32, Trap> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("aleph-main".to_string())
            .stack_size(MAIN_STACK_SIZE)
            .spawn_scoped(scope, || {
                let stdout = std::io::stdout();
                let mut out = BufWriter::new(stdout.lock());
                let stderr = std::io::stderr();
                let mut err = BufWriter::new(stderr.lock());
                let result = {
                    let mut interp = Interp::new(file, args, &mut out, &mut err);
                    interp.interpret_main()
                };
                let _ = out.flush();
                let _ = err.flush();
                result
            })
            .expect("failed to spawn the interpreter thread");
        match handle.join() {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}

/// Execute `main` with explicit writers. Both writers must be `Send` because
/// interpretation runs on the big-stack thread; they are flushed before
/// returning. Used by tests and embedders.
pub fn run_main_with<O: Write + Send, E: Write + Send>(
    file: &ast::File,
    args: Vec<String>,
    out: &mut O,
    err: &mut E,
) -> Result<i32, Trap> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("aleph-main".to_string())
            .stack_size(MAIN_STACK_SIZE)
            .spawn_scoped(scope, || {
                let result = {
                    let mut interp = Interp::new(file, args, out, err);
                    interp.interpret_main()
                };
                let _ = out.flush();
                let _ = err.flush();
                result
            })
            .expect("failed to spawn the interpreter thread");
        match handle.join() {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}

/// Deepest value level [`literal`] expands before printing `...`.
const MAX_LITERAL_DEPTH: usize = 100;

/// One pending step of the iterative value formatter.
enum ShowTask<'a> {
    Value(&'a Value, bool, usize),
    Text(&'static str),
}

/// Render `value` without recursing over its structure.
///
/// `quote_string` quotes a string at this level; children always quote theirs.
/// `max_depth` caps `literal`, whose output is diagnostic text.
fn format_value(value: &Value, quote_string: bool, max_depth: Option<usize>) -> String {
    let mut out = String::new();
    let mut stack = vec![ShowTask::Value(value, quote_string, 0)];
    while let Some(task) = stack.pop() {
        match task {
            ShowTask::Text(text) => out.push_str(text),
            ShowTask::Value(value, quote_string, depth) => match value {
                Value::I64(number) => out.push_str(&number.to_string()),
                Value::F64(number) => out.push_str(&format!("{number:?}")),
                Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
                Value::Str(text) if quote_string => {
                    out.push('"');
                    out.push_str(&ast::escape_str(text));
                    out.push('"');
                }
                Value::Str(text) => out.push_str(text),
                Value::Unit => out.push_str("nil"),
                Value::List(values) => {
                    if max_depth.is_some_and(|max| depth >= max) {
                        out.push_str("...");
                        continue;
                    }
                    out.push('[');
                    stack.push(ShowTask::Text("]"));
                    for (index, item) in values.iter().enumerate().rev() {
                        stack.push(ShowTask::Value(item, true, depth + 1));
                        if index > 0 {
                            stack.push(ShowTask::Text(" "));
                        }
                    }
                }
                Value::Opt(Some(inner)) => {
                    if max_depth.is_some_and(|max| depth >= max) {
                        out.push_str("...");
                        continue;
                    }
                    out.push_str("some ");
                    stack.push(ShowTask::Value(inner, true, depth + 1));
                }
                Value::Opt(None) => out.push_str("none"),
                Value::Res(Ok(inner)) => {
                    if max_depth.is_some_and(|max| depth >= max) {
                        out.push_str("...");
                        continue;
                    }
                    out.push_str("ok ");
                    stack.push(ShowTask::Value(inner, true, depth + 1));
                }
                Value::Res(Err(inner)) => {
                    if max_depth.is_some_and(|max| depth >= max) {
                        out.push_str("...");
                        continue;
                    }
                    out.push_str("err ");
                    stack.push(ShowTask::Value(inner, true, depth + 1));
                }
                Value::Record(name, fields) => {
                    if max_depth.is_some_and(|max| depth >= max) {
                        out.push_str("...");
                        continue;
                    }
                    out.push_str(name);
                    for field in fields.iter().rev() {
                        stack.push(ShowTask::Value(field, true, depth + 1));
                        stack.push(ShowTask::Text(" "));
                    }
                }
                Value::Dict(pairs) => {
                    if max_depth.is_some_and(|max| depth >= max) {
                        out.push_str("...");
                        continue;
                    }
                    out.push_str(lexicon::type_name("dict").expect("dict"));
                    for (key, val) in pairs.iter().rev() {
                        stack.push(ShowTask::Value(val, true, depth + 1));
                        stack.push(ShowTask::Text(" "));
                        stack.push(ShowTask::Value(key, true, depth + 1));
                        stack.push(ShowTask::Text(" "));
                    }
                }
                Value::Json(payload, _) => out.push_str(&json::print(payload)),
                Value::Fn(target) => match target {
                    FnTarget::User(name) | FnTarget::Ctor(name) => {
                        out.push('@');
                        out.push_str(name);
                    }
                    FnTarget::Builtin(name) => {
                        out.push('@');
                        out.push_str(name);
                    }
                    FnTarget::Lam(..) => out.push_str(lexicon::keyword("lambda").expect("lam")),
                },
            },
        }
    }
    out
}

/// `show` from SPEC §5.1: primitives, with list literals quoting strings.
pub fn show(value: &Value) -> String {
    format_value(value, false, None)
}

/// Deterministic Aleph literal text for `value` (ADR-0022).
///
/// Unlike [`show`], a top-level string is quoted. Nested values use prefix
/// syntax without parentheses, which is unambiguous in Aleph. Values nested
/// deeper than [`MAX_LITERAL_DEPTH`] are truncated to `...` so a diagnostic
/// never walks an arbitrarily deep value.
pub fn literal(value: &Value) -> String {
    format_value(value, true, Some(MAX_LITERAL_DEPTH))
}

/// `some payload` when the json payload passes `kind`, else `none`.
fn json_field(value: &Value, kind: fn(&Value) -> bool) -> Value {
    let payload = json::inner(value);
    Value::Opt(kind(payload).then(|| Rc::new(payload.clone())))
}

fn as_i64(value: &Value) -> i64 {
    match value {
        Value::I64(number) => *number,
        _ => panic!("type checker guarantees an i64"),
    }
}

fn as_bool(value: &Value) -> bool {
    match value {
        Value::Bool(flag) => *flag,
        _ => panic!("type checker guarantees a bool"),
    }
}

fn as_str(value: &Value) -> &str {
    match value {
        Value::Str(text) => text,
        _ => panic!("type checker guarantees a str"),
    }
}

fn as_dict(value: &Value) -> &Rc<Vec<(Value, Value)>> {
    match value {
        Value::Dict(pairs) => pairs,
        _ => panic!("type checker guarantees a dict"),
    }
}

/// Binary search for `key` in sorted dict pairs.
fn find_key(pairs: &[(Value, Value)], key: &Value) -> Result<usize, usize> {
    pairs.binary_search_by(|(probe, _)| compare_values(probe, key))
}

fn as_list(value: &Value) -> &Rc<Vec<Value>> {
    match value {
        Value::List(values) => values,
        _ => panic!("type checker guarantees a list"),
    }
}

fn as_fn(value: &Value) -> &FnTarget {
    match value {
        Value::Fn(target) => target,
        _ => panic!("type checker guarantees a function value"),
    }
}

fn result_ok(value: Value) -> Value {
    Value::Res(Ok(Rc::new(value)))
}

fn result_err(message: &str) -> Value {
    Value::Res(Err(Rc::new(Value::Str(Rc::from(message)))))
}

fn clamp_index(value: i64, len: usize) -> usize {
    if value < 0 {
        0
    } else {
        (value as u64).min(len as u64) as usize
    }
}

fn compare(op: BinOp, left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::I64(a), Value::I64(b)) => compare_ordering(op, Some(a.cmp(b))),
        (Value::F64(a), Value::F64(b)) => compare_ordering(op, a.partial_cmp(b)),
        _ => panic!("type checker guarantees matching comparable operands"),
    }
}

fn compare_ordering(op: BinOp, ordering: Option<Ordering>) -> bool {
    let Some(ordering) = ordering else {
        // IEEE comparisons with NaN are all false.
        return false;
    };
    match op {
        BinOp::Lt => ordering == Ordering::Less,
        BinOp::Le => ordering != Ordering::Greater,
        BinOp::Gt => ordering == Ordering::Greater,
        BinOp::Ge => ordering != Ordering::Less,
        _ => unreachable!(),
    }
}

pub(crate) fn compare_values(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::I64(a), Value::I64(b)) => a.cmp(b),
        // Every NaN sorts after +inf whatever its sign bit: the sign of
        // `0.0 / 0.0` differs between x86-64 and arm64 (ADR-0024).
        (Value::F64(a), Value::F64(b)) => match (a.is_nan(), b.is_nan()) {
            (false, false) => a.total_cmp(b),
            (left, right) => left.cmp(&right),
        },
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Str(a), Value::Str(b)) => a.as_bytes().cmp(b.as_bytes()),
        _ => Ordering::Equal,
    }
}

fn parse_int(text: &str) -> Option<i64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

fn parse_float(text: &str) -> Option<f64> {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mut parts = unsigned.split('.');
    let (Some(whole), Some(fraction), None) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if whole.is_empty()
        || fraction.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    text.parse::<f64>().ok()
}

/// Resolve `path` to an absolute canonical path without requiring the leaf to
/// exist.
///
/// `..` components are rejected. If the path exists (including a symlink), it
/// must canonicalize successfully, so a dangling symlink returns `None`.
/// Otherwise the longest existing ancestor is canonicalized and the remaining
/// components are appended.
fn resolve_lenient(path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    let original = Path::new(path);
    if original
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    if fs::symlink_metadata(original).is_ok() {
        return fs::canonicalize(original).ok();
    }

    let components: Vec<Component> = original
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .collect();
    for length in (0..components.len()).rev() {
        let mut ancestor = PathBuf::new();
        for component in &components[..length] {
            ancestor.push(component.as_os_str());
        }
        if length == 0 {
            ancestor.push(".");
        }
        if fs::symlink_metadata(&ancestor).is_ok() {
            let mut resolved = fs::canonicalize(&ancestor).ok()?;
            for component in &components[length..] {
                resolved.push(component.as_os_str());
            }
            return Some(resolved);
        }
    }
    None
}

/// Run `curl` without a shell and map its result to `fetch`'s result value.
fn fetch(url: &str) -> Value {
    const LIMIT: usize = 16_777_216;
    let mut child = match Command::new("curl")
        .args([
            "-q",
            "-g",
            "-sS",
            "--fail",
            "--max-time",
            "30",
            "--max-redirs",
            "0",
            "--proto",
            "=http,https",
            "--max-filesize",
            "16777216",
            "--",
            url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return result_err("fetch: curl not found")
        }
        Err(_) => return result_err("fetch: curl failed"),
    };

    let mut stdout = child.stdout.take().expect("curl stdout is piped");
    let stderr = child.stderr.take().expect("curl stderr is piped");
    let stderr_thread = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut stderr = stderr;
        let _ = stderr.read_to_end(&mut buffer);
        buffer
    });

    // Read at most one byte past the limit; anything more than the limit means
    // the response is too large and curl must be killed.
    let mut body = Vec::new();
    let read_failed = stdout
        .by_ref()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut body)
        .is_err();
    let too_large = body.len() > LIMIT;
    if too_large || read_failed {
        let _ = child.kill();
    }
    let status = child.wait();
    let stderr = stderr_thread.join().unwrap_or_default();

    if too_large {
        return result_err("fetch: response too large");
    }
    if read_failed {
        return result_err("fetch: io error"); // as Tlön
    }

    let error_text = curl_error_text(&stderr);
    let failed = match &status {
        Ok(status) => !status.success(),
        Err(_) => true,
    };
    if failed {
        if status.as_ref().ok().and_then(|status| status.code()) == Some(63) {
            return result_err("fetch: response too large");
        }
        if error_text.is_empty() {
            return result_err("fetch: curl failed");
        }
        return result_err(&format!("fetch: {error_text}"));
    }
    match String::from_utf8(body) {
        Ok(body) => result_ok(Value::Str(Rc::from(body.as_str()))),
        Err(_) => result_err("fetch: body is not valid UTF-8"),
    }
}

/// The first line of curl's stderr with its `curl: (N) ` prefix removed.
fn curl_error_text(stderr: &[u8]) -> String {
    let first = stderr.split(|byte| *byte == b'\n').next().unwrap_or(&[]);
    // Not UTF-8 -> empty, so the caller says `curl failed` (as Tlön).
    let text = std::str::from_utf8(first).unwrap_or("");
    let text = text.trim_end_matches([' ', '\t', '\r']);
    let Some(rest) = text.strip_prefix("curl: (") else {
        return text.to_string();
    };
    let Some(close) = rest.find(')') else {
        return text.to_string();
    };
    rest[close + 1..]
        .strip_prefix(' ')
        .unwrap_or(&rest[close + 1..])
        .to_string()
}

/// The io error kind as lowercase words (`not found`, `permission denied`).
fn io_error_text(error: &std::io::Error) -> String {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::NotFound => "not found".to_string(),
        ErrorKind::PermissionDenied => "permission denied".to_string(),
        ErrorKind::AlreadyExists => "already exists".to_string(),
        ErrorKind::InvalidInput => "invalid input".to_string(),
        ErrorKind::InvalidData => "invalid data".to_string(),
        ErrorKind::IsADirectory => "is a directory".to_string(),
        ErrorKind::NotADirectory => "not a directory".to_string(),
        ErrorKind::DirectoryNotEmpty => "directory not empty".to_string(),
        ErrorKind::ReadOnlyFilesystem => "read-only filesystem".to_string(),
        ErrorKind::WouldBlock => "would block".to_string(),
        ErrorKind::UnexpectedEof => "unexpected eof".to_string(),
        ErrorKind::OutOfMemory => "out of memory".to_string(),
        _ => "io error".to_string(),
    }
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

    fn call(source: &str, name: &str, args: Vec<Value>) -> Result<Value, Trap> {
        let file = parse_ok(source);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut interp = Interp::new(&file, Vec::new(), &mut out, &mut err);
        interp.call_fn(name, args)
    }

    fn run(source: &str, args: &[&str]) -> (Result<i32, Trap>, String, String) {
        let file = parse_ok(source);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = run_main_with(
            &file,
            args.iter().map(|arg| arg.to_string()).collect(),
            &mut out,
            &mut err,
        );
        (
            result,
            String::from_utf8(out).expect("utf-8 stdout"),
            String::from_utf8(err).expect("utf-8 stderr"),
        )
    }

    fn probe(source: &str) -> Value {
        call(source, "probe", Vec::new()).expect("probe succeeds")
    }

    fn i64_value(value: i64) -> Value {
        Value::I64(value)
    }

    fn str_value(value: &str) -> Value {
        Value::Str(Rc::from(value))
    }

    fn list(values: Vec<Value>) -> Value {
        Value::List(Rc::new(values))
    }

    fn some(value: Value) -> Value {
        Value::Opt(Some(Rc::new(value)))
    }

    fn ok(value: Value) -> Value {
        result_ok(value)
    }

    fn err(value: Value) -> Value {
        Value::Res(Err(Rc::new(value)))
    }

    fn trap_code(result: Result<Value, Trap>) -> &'static str {
        result.expect_err("expected a trap").code
    }

    /// A sum-like record chain `s (s (... z))` with `depth` constructors.
    fn deep_sum(depth: usize) -> Value {
        let s: Rc<str> = Rc::from("s");
        let z: Rc<str> = Rc::from("z");
        let mut value = Value::Record(z, Rc::new(Vec::new()));
        for _ in 0..depth {
            value = Value::Record(s.clone(), Rc::new(vec![value]));
        }
        value
    }

    #[test]
    fn deep_value_drop_is_iterative() {
        drop(deep_sum(1_000_000));
    }

    #[test]
    fn deep_value_equality_and_literal_are_iterative() {
        let value = deep_sum(1_000_000);
        let copy = value.clone();
        assert!(values_equal(&value, &copy));
        let text = literal(&value);
        assert!(text.ends_with("..."), "literal was not truncated");
        assert!(
            text.len() < 1_000,
            "literal was not capped: {} bytes",
            text.len()
        );
        drop(copy);
        drop(value);
    }

    #[test]
    fn values_equal_is_structural() {
        assert!(values_equal(&i64_value(1), &i64_value(1)));
        assert!(values_equal(&Value::F64(1.5), &Value::F64(1.5)));
        assert!(!values_equal(&Value::F64(f64::NAN), &Value::F64(f64::NAN)));
        assert!(values_equal(&str_value("a"), &str_value("a")));
        assert!(!values_equal(&str_value("a"), &i64_value(1)));
        assert!(values_equal(
            &list(vec![i64_value(1), str_value("x")]),
            &list(vec![i64_value(1), str_value("x")])
        ));
        assert!(!values_equal(
            &list(vec![i64_value(1)]),
            &list(vec![i64_value(1), i64_value(2)])
        ));
        assert!(values_equal(&some(i64_value(1)), &some(i64_value(1))));
        assert!(!values_equal(&some(i64_value(1)), &Value::Opt(None)));
        assert!(values_equal(&ok(i64_value(1)), &ok(i64_value(1))));
        assert!(!values_equal(&ok(i64_value(1)), &err(str_value("x"))));
        assert!(values_equal(
            &Value::Record(Rc::from("p"), Rc::new(vec![i64_value(1)])),
            &Value::Record(Rc::from("p"), Rc::new(vec![i64_value(1)]))
        ));
        assert!(!values_equal(
            &Value::Record(Rc::from("p"), Rc::new(vec![i64_value(1)])),
            &Value::Record(Rc::from("q"), Rc::new(vec![i64_value(1)]))
        ));
        assert!(values_equal(
            &Value::Fn(FnTarget::User("f".to_string())),
            &Value::Fn(FnTarget::User("f".to_string()))
        ));
    }

    #[test]
    fn lambdas_run_in_map_filter_fold_map2_with_captures() {
        let cases = [
            ("fn probe -> list i64\n let k 10\n map lam x * x k [1 2 3]\n", "[10 20 30]"),
            ("fn probe -> list i64\n let k 2\n filter lam x > x k [1 2 3 4]\n", "[3 4]"),
            ("fn probe -> i64\n fold lam acc x - acc x 100 [1 2 3]\n", "94"),
            (
                "fn probe -> list str\n let sep \":\"\n map2 lam a b cat cat show a sep b [1 2 3] [\"x\" \"y\"]\n",
                "[\"1:x\" \"2:y\"]",
            ),
            // An inner lambda captures an outer lambda binder.
            ("fn probe -> list list i64\n map lam x map lam y + x y [10 20] [1 2]\n", "[[11 21] [12 22]]"),
            // A match binder is captured.
            (
                "type box | full v:i64 | hole\nfn probe -> list i64\n match full 5 full v map lam x + x v [1 2] hole empty i64\n",
                "[6 7]",
            ),
            // A user function is called from a lambda body.
            ("fn sq x:i64 -> i64\n * x x\nfn probe -> i64\n fold lam a x + a sq x 0 [1 2 3]\n", "14"),
        ];
        for (source, want) in cases {
            assert_eq!(literal(&probe(source)), want, "{source}");
        }
    }

    #[test]
    fn lambda_values_capture_by_value_once() {
        // Each evaluation of `lam` captures the current value of `n`.
        let source = "fn add n:i64 xs:list i64 -> list i64\n map lam x + x n xs\nfn probe -> list list i64\n [add 1 [0] add 2 [0]]\n";
        assert_eq!(literal(&probe(source)), "[[1] [2]]");
        let file = parse_ok("fn probe xs:list i64 -> list i64\n map lam x x xs\n");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut interp = Interp::new(&file, Vec::new(), &mut out, &mut err);
        assert_eq!(interp.lams.len(), 1);
        let ast::ExprKind::Call { args, .. } = &file.fns[0].result.kind else {
            panic!("expected call");
        };
        let value = interp.eval_expr(&args[0], &Env::new()).expect("lam value");
        assert_eq!(literal(&value), "lam");
    }

    #[test]
    fn traps_inside_lambdas_point_at_the_body() {
        let r001 = call(
            "fn probe -> list i64\n map lam x + x 9223372036854775807 [0 1]\n",
            "probe",
            Vec::new(),
        );
        assert_eq!(trap_code(r001), "R001");
        let r002 = call(
            "fn probe -> list i64\n let z 0\n map lam x / x z [1]\n",
            "probe",
            Vec::new(),
        )
        .expect_err("division by zero");
        assert_eq!(r002.code, "R002");
        assert_eq!((r002.span.line, r002.span.col), (3, 12));
        // Unbounded recursion through a lambda; `run` uses the main stack size.
        let (r004, _, _) = run(
            "fn f n:i64 -> list i64\n map lam x len f + x 1 [n]\nfn main -> unit\n print show f 0\n",
            &[],
        );
        assert_eq!(r004.expect_err("recursion traps").code, "R004");
    }

    #[test]
    fn lambdas_may_print_and_run_from_main() {
        let (result, out, _) = run(
            "fn main -> unit\n let tag \"n=\"\n do print show len map lam x print cat tag show x [1 2]\n nil\n",
            &[],
        );
        assert_eq!(result.expect("runs"), 0);
        assert_eq!(out, "n=1\nn=2\n2\n");
    }

    #[test]
    fn variants_are_records_named_by_variant_and_match_dispatches() {
        let shape = "type shape | circle r:i64 | rect w:i64 h:i64 | dot\n";
        assert!(values_equal(
            &probe(&format!("{shape}fn probe -> shape\n rect 2 3\n")),
            &Value::Record(Rc::from("rect"), Rc::new(vec![i64_value(2), i64_value(3)]))
        ));
        assert_eq!(
            literal(&probe(&format!(
                "{shape}fn probe -> list shape\n [circle 1 dot]\n"
            ))),
            "[circle 1 dot]"
        );
        let area = format!(
            "{shape}fn area s:shape -> i64\n match s circle r * 3 * r r rect w h * w h dot 0\n"
        );
        for (arg, want) in [("circle 2", 12), ("rect 2 5", 10), ("dot", 0)] {
            let source = format!("{area}fn probe -> i64\n area {arg}\n");
            assert!(values_equal(&probe(&source), &i64_value(want)), "{arg}");
        }
    }

    #[test]
    fn literals_and_record_fields() {
        assert!(values_equal(&probe("fn probe -> i64\n 7\n"), &i64_value(7)));
        assert!(values_equal(
            &probe("fn probe -> f64\n 1.25\n"),
            &Value::F64(1.25)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n true\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> unit\n nil\n"),
            &Value::Unit
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n \"hi\"\n"),
            &str_value("hi")
        ));
        assert!(values_equal(
            &probe("type point x:i64 y:i64\nfn probe -> i64\n .y point 3 4\n"),
            &i64_value(4)
        ));
    }

    #[test]
    fn len_at_push_slice_rev() {
        assert!(values_equal(
            &probe("fn probe -> i64\n len [1 2 3]\n"),
            &i64_value(3)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n len empty i64\n"),
            &i64_value(0)
        ));
        assert!(values_equal(
            &probe("fn probe -> opt i64\n at [10 20] 1\n"),
            &some(i64_value(20))
        ));
        assert!(values_equal(
            &probe("fn probe -> opt i64\n at [10 20] -1\n"),
            &Value::Opt(None)
        ));
        assert!(values_equal(
            &probe("fn probe -> opt i64\n at [10 20] 2\n"),
            &Value::Opt(None)
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n push [1] 2\n"),
            &list(vec![i64_value(1), i64_value(2)])
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n slice [1 2 3 4] 1 3\n"),
            &list(vec![i64_value(2), i64_value(3)])
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n slice [1 2 3 4] -5 99\n"),
            &list(vec![i64_value(1), i64_value(2), i64_value(3), i64_value(4)])
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n slice [1 2 3] 2 2\n"),
            &list(Vec::new())
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n slice [1 2 3] 2 1\n"),
            &list(Vec::new())
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n rev [1 2 3]\n"),
            &list(vec![i64_value(3), i64_value(2), i64_value(1)])
        ));
    }

    #[test]
    fn sort_and_range_and_sum() {
        assert!(values_equal(
            &probe("fn probe -> list i64\n sort [3 1 2]\n"),
            &list(vec![i64_value(1), i64_value(2), i64_value(3)])
        ));
        assert!(values_equal(
            &probe("fn probe -> list f64\n sort [2.5 -1.0 0.5]\n"),
            &list(vec![Value::F64(-1.0), Value::F64(0.5), Value::F64(2.5)])
        ));
        assert!(values_equal(
            &probe("fn probe -> list bool\n sort [true false true]\n"),
            &list(vec![
                Value::Bool(false),
                Value::Bool(true),
                Value::Bool(true)
            ])
        ));
        assert!(values_equal(
            &probe("fn probe -> list str\n sort [\"b\" \"a\" \"c\"]\n"),
            &list(vec![str_value("a"), str_value("b"), str_value("c")])
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n range 0 3\n"),
            &list(vec![i64_value(0), i64_value(1), i64_value(2)])
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n range 3 3\n"),
            &list(Vec::new())
        ));
        assert!(values_equal(
            &probe("fn probe -> list i64\n range -2 1\n"),
            &list(vec![i64_value(-2), i64_value(-1), i64_value(0)])
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n sum [1 2 3]\n"),
            &i64_value(6)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n sum empty i64\n"),
            &i64_value(0)
        ));
    }

    #[test]
    fn higher_order_builtins_call_user_functions_ctors_and_builtins() {
        let source = "type box v:i64\n\
                      fn inc n:i64 -> i64\n + n 1\n\
                      fn positive n:i64 -> bool\n > n 0\n\
                      fn add a:i64 b:i64 -> i64\n + a b\n\
                      fn probe -> list i64\n map @inc [1 2]\n";
        assert!(values_equal(
            &probe(source),
            &list(vec![i64_value(2), i64_value(3)])
        ));
        assert!(values_equal(
            &call(
                "fn positive n:i64 -> bool\n > n 0\nfn probe -> list i64\n filter @positive [-1 0 2 3]\n",
                "probe",
                Vec::new()
            )
            .unwrap(),
            &list(vec![i64_value(2), i64_value(3)])
        ));
        assert!(values_equal(
            &call(
                "fn add a:i64 b:i64 -> i64\n + a b\nfn probe -> i64\n fold @add 0 [1 2 3]\n",
                "probe",
                Vec::new()
            )
            .unwrap(),
            &i64_value(6)
        ));
        assert!(values_equal(
            &call(
                "type box v:i64\nfn probe -> list box\n map @box [1 2]\n",
                "probe",
                Vec::new()
            )
            .unwrap(),
            &list(vec![
                Value::Record(Rc::from("box"), Rc::new(vec![i64_value(1)])),
                Value::Record(Rc::from("box"), Rc::new(vec![i64_value(2)]))
            ])
        ));
        assert!(values_equal(
            &call(
                "fn flip n:i64 -> i64\n neg n\nfn probe -> list i64\n map @flip [1 -2]\n",
                "probe",
                Vec::new()
            )
            .unwrap(),
            &list(vec![i64_value(-1), i64_value(2)])
        ));
    }

    #[test]
    fn string_builtins() {
        assert!(values_equal(
            &probe("fn probe -> str\n cat \"ab\" \"cd\"\n"),
            &str_value("abcd")
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n count \"abc\"\n"),
            &i64_value(3)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n count \"é\"\n"),
            &i64_value(2)
        ));
        assert!(values_equal(
            &probe("fn probe -> list str\n split \"a,b\" \",\"\n"),
            &list(vec![str_value("a"), str_value("b")])
        ));
        assert!(values_equal(
            &probe("fn probe -> list str\n split \"abc\" \"\"\n"),
            &list(vec![str_value("a"), str_value("b"), str_value("c")])
        ));
        assert!(values_equal(
            &probe("fn probe -> list str\n split \"\" \"\"\n"),
            &list(Vec::new())
        ));
        assert!(values_equal(
            &probe("fn probe -> list str\n split \"\" \",\"\n"),
            &list(vec![str_value("")])
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n join [\"a\" \"b\"] \"-\"\n"),
            &str_value("a-b")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n trim \"  hi \\t\"\n"),
            &str_value("hi")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n upper \"aBc!\"\n"),
            &str_value("ABC!")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n lower \"aBc!\"\n"),
            &str_value("abc!")
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n contains \"abc\" \"b\"\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n contains \"abc\" \"z\"\n"),
            &Value::Bool(false)
        ));
    }

    #[test]
    fn int_and_float_parsing() {
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"42\"\n"),
            &ok(i64_value(42))
        ));
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"-7\"\n"),
            &ok(i64_value(-7))
        ));
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"007\"\n"),
            &ok(i64_value(7))
        ));
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"\"\n"),
            &err(str_value("invalid int"))
        ));
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"+1\"\n"),
            &err(str_value("invalid int"))
        ));
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"1.0\"\n"),
            &err(str_value("invalid int"))
        ));
        assert!(values_equal(
            &probe("fn probe -> res i64 str\n int \"999999999999999999999999999\"\n"),
            &err(str_value("invalid int"))
        ));
        assert!(values_equal(
            &probe("fn probe -> res f64 str\n float \"1.5\"\n"),
            &ok(Value::F64(1.5))
        ));
        assert!(values_equal(
            &probe("fn probe -> res f64 str\n float \"-0.50\"\n"),
            &ok(Value::F64(-0.5))
        ));
        for bad in ["1", "1.", ".5", "1e3", "", "1.2.3"] {
            let source = format!("fn probe -> res f64 str\n float \"{bad}\"\n");
            assert!(
                values_equal(&probe(&source), &err(str_value("invalid float"))),
                "float \"{bad}\" should be invalid"
            );
        }
    }

    #[test]
    fn show_formatting() {
        assert!(values_equal(
            &probe("fn probe -> str\n show 42\n"),
            &str_value("42")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show 1.0\n"),
            &str_value("1.0")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show 1.5\n"),
            &str_value("1.5")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show true\n"),
            &str_value("true")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show \"hi\"\n"),
            &str_value("hi")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show [1 2 3]\n"),
            &str_value("[1 2 3]")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show empty i64\n"),
            &str_value("[]")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show [\"a b\" \"c\\nd\"]\n"),
            &str_value("[\"a b\" \"c\\nd\"]")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show [1.0 2.5]\n"),
            &str_value("[1.0 2.5]")
        ));
        assert!(values_equal(
            &probe("fn probe -> str\n show [true false]\n"),
            &str_value("[true false]")
        ));
    }

    #[test]
    fn args_print_and_eprint() {
        let source =
            "fn main -> unit\n do print \"out\"\n do eprint \"err\"\n do print show args\n nil\n";
        let (result, out, err) = run(source, &["a", "b"]);
        assert_eq!(result.expect("main succeeds"), 0);
        assert_eq!(out, "out\n[\"a\" \"b\"]\n");
        assert_eq!(err, "err\n");
    }

    #[test]
    fn checked_arithmetic_and_comparisons() {
        assert!(values_equal(
            &probe("fn probe -> i64\n + 2 3\n"),
            &i64_value(5)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n - 2 3\n"),
            &i64_value(-1)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n * 2 3\n"),
            &i64_value(6)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n / 7 2\n"),
            &i64_value(3)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n % 7 2\n"),
            &i64_value(1)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n neg 3\n"),
            &i64_value(-3)
        ));
        assert!(values_equal(
            &probe("fn probe -> f64\n / 1.0 0.0\n"),
            &Value::F64(f64::INFINITY)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n == 1 1\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n != 1 1\n"),
            &Value::Bool(false)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n < 1 2\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n <= 2 2\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n > 1 2\n"),
            &Value::Bool(false)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n >= 2 2\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n == \"a\" \"a\"\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n not false\n"),
            &Value::Bool(true)
        ));
    }

    #[test]
    fn integer_overflow_traps() {
        for source in [
            "fn probe -> i64\n + 9223372036854775807 1\n",
            "fn probe -> i64\n - -9223372036854775808 1\n",
            "fn probe -> i64\n * 9223372036854775807 2\n",
            "fn probe -> i64\n neg -9223372036854775808\n",
            "fn probe -> i64\n / -9223372036854775808 -1\n",
            "fn probe -> i64\n % -9223372036854775808 -1\n",
            "fn probe -> i64\n sum [9223372036854775807 1]\n",
        ] {
            assert_eq!(
                trap_code(call(source, "probe", Vec::new())),
                "R001",
                "{source}"
            );
        }
    }

    #[test]
    fn division_by_zero_traps() {
        for source in ["fn probe -> i64\n / 1 0\n", "fn probe -> i64\n % 1 0\n"] {
            assert_eq!(
                trap_code(call(source, "probe", Vec::new())),
                "R002",
                "{source}"
            );
        }
    }

    #[test]
    fn range_allocation_limit_traps_with_r005() {
        for source in [
            "fn probe -> list i64\n range 0 9223372036854775807\n",
            "fn probe -> list i64\n range -9223372036854775808 9223372036854775807\n",
        ] {
            let trap = call(source, "probe", Vec::new()).expect_err("range traps");
            assert_eq!(trap.code, "R005", "{source}");
            assert_eq!(trap.msg, "allocation limit exceeded");
        }
    }

    #[test]
    fn try_returns_early_from_the_enclosing_function() {
        let source = "fn next r:res i64 str -> res i64 str\n let x try r\n ok + x 1\n";
        assert!(values_equal(
            &call(source, "next", vec![ok(i64_value(41))]).unwrap(),
            &ok(i64_value(42))
        ));
        assert!(values_equal(
            &call(source, "next", vec![err(str_value("boom"))]).unwrap(),
            &err(str_value("boom"))
        ));

        let program = "fn next r:res i64 str -> res i64 str\n let x try r\n ok + x 1\n\
                       fn g -> res i64 str\n next err \"boom\"\n\
                       fn main -> res unit str\n let value try g\n do print show value\n ok nil\n";
        let (result, out, err) = run(program, &[]);
        assert_eq!(result.expect("main returns err"), 1);
        assert_eq!(out, "");
        assert_eq!(err, "boom\n");
    }

    #[test]
    fn try_early_return_propagates_through_nested_expressions() {
        let arithmetic = "fn next r:res i64 str -> res i64 str\n let y + try r 1\n ok y\n";
        assert!(values_equal(
            &call(arithmetic, "next", vec![err(str_value("boom"))]).unwrap(),
            &err(str_value("boom"))
        ));
        assert!(values_equal(
            &call(arithmetic, "next", vec![ok(i64_value(41))]).unwrap(),
            &ok(i64_value(42))
        ));

        let list_source = "fn next r:res i64 str -> res list i64 str\n let ys [try r]\n ok ys\n";
        assert!(values_equal(
            &call(list_source, "next", vec![err(str_value("boom"))]).unwrap(),
            &err(str_value("boom"))
        ));

        let argument =
            "fn id n:i64 -> i64\n n\nfn next r:res i64 str -> res i64 str\n ok id try r\n";
        assert!(values_equal(
            &call(argument, "next", vec![err(str_value("boom"))]).unwrap(),
            &err(str_value("boom"))
        ));
    }

    #[test]
    fn f64_comparisons_follow_ieee_with_nan() {
        for source in [
            "fn probe -> bool\n < / 0.0 0.0 1.0\n",
            "fn probe -> bool\n <= / 0.0 0.0 1.0\n",
            "fn probe -> bool\n > / 0.0 0.0 1.0\n",
            "fn probe -> bool\n >= / 0.0 0.0 1.0\n",
            "fn probe -> bool\n == / 0.0 0.0 / 0.0 0.0\n",
        ] {
            assert!(
                values_equal(&probe(source), &Value::Bool(false)),
                "{source}"
            );
        }
    }

    #[test]
    fn short_circuit_and_if_evaluate_only_what_they_need() {
        assert!(values_equal(
            &probe("fn probe -> bool\n and false == / 1 0 0\n"),
            &Value::Bool(false)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n or true == / 1 0 0\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n if true 1 / 1 0\n"),
            &i64_value(1)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n if false / 1 0 2\n"),
            &i64_value(2)
        ));
    }

    #[test]
    fn opt_result_helpers() {
        assert!(values_equal(
            &probe("fn probe -> i64\n else some 5 0\n"),
            &i64_value(5)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n else none i64 7\n"),
            &i64_value(7)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n else ok 3 0\n"),
            &i64_value(3)
        ));
        assert!(values_equal(
            &probe("fn probe -> i64\n else err \"x\" 9\n"),
            &i64_value(9)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n has some 1\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n has none i64\n"),
            &Value::Bool(false)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n isok ok 1\n"),
            &Value::Bool(true)
        ));
        assert!(values_equal(
            &probe("fn probe -> bool\n isok err \"x\"\n"),
            &Value::Bool(false)
        ));
    }

    #[test]
    fn trap_to_diagnostic_has_the_node_and_position() {
        let trap = call("fn probe -> i64\n / 1 0\n", "probe", Vec::new()).unwrap_err();
        let diagnostic = trap.to_diagnostic("a.aph");
        assert_eq!(diagnostic.code, "R002");
        assert_eq!(diagnostic.msg, "division by zero");
        assert_eq!(diagnostic.file, "a.aph");
        assert_eq!(diagnostic.line, 2);
        assert_eq!(diagnostic.col, 2);
        assert_eq!(diagnostic.node.as_deref(), Some("n3"));
    }

    #[test]
    fn main_returning_err_writes_stderr_and_exits_one() {
        let (result, out, err) = run("fn main -> res unit str\n err \"bad\"\n", &[]);
        assert_eq!(result.expect("no trap"), 1);
        assert_eq!(out, "");
        assert_eq!(err, "bad\n");
    }

    #[test]
    fn main_returning_unit_and_ok_exit_zero() {
        assert_eq!(run("fn main -> unit\n nil\n", &[]).0.expect("no trap"), 0);
        assert_eq!(
            run("fn main -> res unit str\n ok nil\n", &[])
                .0
                .expect("no trap"),
            0
        );
    }

    #[test]
    fn recursion_succeeds_below_the_depth_limit() {
        let source = "fn down n:i64 -> i64\n if == n 0 0 down - n 1\n\
                      fn main -> unit\n print show down 5000\n";
        let (result, out, err) = run(source, &[]);
        assert_eq!(result.expect("recursion succeeds"), 0);
        assert_eq!(out, "0\n");
        assert_eq!(err, "");
    }

    #[test]
    fn deep_recursion_traps_with_r004() {
        let source = "fn down n:i64 -> i64\n if == n 0 0 down - n 1\n\
                      fn main -> unit\n print show down 20000\n";
        let (result, _, _) = run(source, &[]);
        let trap = result.expect_err("recursion traps");
        assert_eq!(trap.code, "R004");
        assert_eq!(trap.msg, "stack overflow");
    }

    #[test]
    fn deep_nested_recursion_traps_without_sigsegv() {
        let mut body = String::from("deep - n 1");
        for _ in 0..250 {
            body = format!("+ 1 {body}");
        }
        let source =
            format!("fn deep n:i64 -> i64\n {body}\nfn main -> unit\n print show deep 0\n");
        let (result, _, _) = run(&source, &[]);
        let trap = result.expect_err("deep nested recursion traps");
        assert_eq!(trap.code, "R004");
        assert_eq!(trap.msg, "stack overflow");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("aleph-interp-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        fs::canonicalize(&path).expect("canonical temp dir")
    }

    fn path_string(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn read_and_write_inside_a_declared_prefix() {
        let dir = temp_dir("rw");
        fs::write(dir.join("in.txt"), "hello").unwrap();
        let prefix = path_string(&dir);
        let input = path_string(&dir.join("in.txt"));
        let output = path_string(&dir.join("out.txt"));
        let source = format!(
            "use fs.read \"{prefix}\"\nuse fs.write \"{prefix}\"\n\
             fn main -> res unit str\n let text try read \"{input}\"\n do print text\n let r try write \"{output}\" text\n let back try read \"{output}\"\n do print back\n ok nil\n"
        );
        let (result, out, err) = run(&source, &[]);
        assert_eq!(result.expect("no trap"), 0, "err: {err}");
        assert_eq!(out, "hello\nhello\n");
        assert_eq!(fs::read_to_string(dir.join("out.txt")).unwrap(), "hello");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_and_write_outside_the_prefix_trap_with_r003() {
        let dir = temp_dir("outside");
        let other = temp_dir("other");
        let prefix = path_string(&dir);
        let read_path = path_string(&other.join("in.txt"));
        fs::write(&read_path, "x").unwrap();
        let write_path = path_string(&other.join("out.txt"));

        let read_source = format!(
            "use fs.read \"{prefix}\"\nfn main -> res unit str\n let text try read \"{read_path}\"\n ok nil\n"
        );
        let (result, _, _) = run(&read_source, &[]);
        assert_eq!(result.expect_err("read traps").code, "R003");

        let write_source = format!(
            "use fs.write \"{prefix}\"\nfn main -> res unit str\n let r try write \"{write_path}\" \"x\"\n ok nil\n"
        );
        let (result, _, _) = run(&write_source, &[]);
        assert_eq!(result.expect_err("write traps").code, "R003");

        let no_use =
            format!("fn main -> res unit str\n let text try read \"{read_path}\"\n ok nil\n");
        let (result, _, _) = run(&no_use, &[]);
        assert_eq!(result.expect_err("missing use traps").code, "R003");

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&other);
    }

    #[test]
    fn read_reports_io_errors_for_missing_files_inside_the_prefix() {
        let dir = temp_dir("missing");
        let prefix = path_string(&dir);
        let missing = path_string(&dir.join("nope.txt"));
        let source =
            format!("use fs.read \"{prefix}\"\nfn probe -> res str str\n read \"{missing}\"\n");
        assert!(values_equal(
            &call(&source, "probe", Vec::new()).unwrap(),
            &err(str_value("not found"))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nested_missing_path_inside_the_prefix_reports_not_found() {
        let dir = temp_dir("nested-missing");
        let prefix = path_string(&dir);
        let missing = path_string(&dir.join("a/b/c.txt"));
        let source =
            format!("use fs.read \"{prefix}\"\nfn probe -> res str str\n read \"{missing}\"\n");
        assert!(values_equal(
            &call(&source, "probe", Vec::new()).unwrap(),
            &err(str_value("not found"))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reading_a_directory_reports_is_a_directory() {
        let dir = temp_dir("dir-read");
        fs::create_dir(dir.join("subdir")).unwrap();
        let prefix = path_string(&dir);
        let subdir = path_string(&dir.join("subdir"));
        let source =
            format!("use fs.read \"{prefix}\"\nfn probe -> res str str\n read \"{subdir}\"\n");
        assert!(values_equal(
            &call(&source, "probe", Vec::new()).unwrap(),
            &err(str_value("is a directory"))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn dangling_symlink_write_traps_and_creates_nothing_outside() {
        use std::os::unix::fs::symlink;

        let dir = temp_dir("dangling");
        let outside = temp_dir("dangling-outside");
        let target = outside.join("created.txt");
        let link = dir.join("link.txt");
        symlink(&target, &link).unwrap();

        let prefix = path_string(&dir);
        let link_path = path_string(&link);
        let source = format!(
            "use fs.write \"{prefix}\"\nfn main -> res unit str\n let r try write \"{link_path}\" \"x\"\n ok nil\n"
        );
        let (result, _, _) = run(&source, &[]);
        assert_eq!(result.expect_err("dangling symlink traps").code, "R003");
        assert!(
            !target.exists(),
            "nothing may be created outside the prefix"
        );

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn relative_write_prefix_works_on_the_first_run() {
        let name = format!("aleph-interp-relative-{}.txt", std::process::id());
        let _ = fs::remove_file(&name);
        let source = format!(
            "use fs.write \"{name}\"\nfn main -> res unit str\n let r try write \"{name}\" \"x\"\n ok nil\n"
        );
        let (result, _, err) = run(&source, &[]);
        assert_eq!(result.expect("write succeeds"), 0, "stderr: {err}");
        assert_eq!(fs::read_to_string(&name).unwrap(), "x");
        let _ = fs::remove_file(&name);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escaping_the_prefix_traps_with_r003() {
        use std::os::unix::fs::symlink;

        let dir = temp_dir("symlink");
        let outside = temp_dir("symlink-outside");
        let secret = outside.join("secret.txt");
        fs::write(&secret, "secret").unwrap();
        let link = dir.join("link.txt");
        symlink(&secret, &link).unwrap();

        let prefix = path_string(&dir);
        let link_path = path_string(&link);
        let source = format!(
            "use fs.read \"{prefix}\"\nfn main -> res unit str\n let text try read \"{link_path}\"\n do print text\n ok nil\n"
        );
        let (result, out, _) = run(&source, &[]);
        let trap = result.expect_err("symlink escape traps");
        assert_eq!(trap.code, "R003");
        assert_eq!(out, "");

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn spec_12_fib_program_prints_6765() {
        let source = "fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n + a b\n\
                      fn fib n:i64 -> i64\n doc \"nth fibonacci number\"\n ex fib 0 = 0\n ex fib 10 = 55\n if < n 2 n + fib - n 1 fib - n 2\n\
                      fn total xs:list i64 -> i64\n ex total [1 2 3] = 6\n ex total empty i64 = 0\n fold @add 0 xs\n\
                      fn main -> unit\n print show fib 20\n";
        let (result, out, err) = run(source, &[]);
        assert_eq!(result.expect("no trap"), 0);
        assert_eq!(out, "6765\n");
        assert_eq!(err, "");
    }

    #[test]
    fn spec_12_words_program_reads_a_temp_file() {
        let dir = temp_dir("words");
        fs::write(dir.join("in.txt"), "a b c\n").unwrap();
        let prefix = path_string(&dir);
        let input = path_string(&dir.join("in.txt"));
        let source = format!(
            "use fs.read \"{prefix}\"\n\
             fn words s:str -> i64\n ex words \"a b c\" = 3\n len split trim s \" \"\n\
             fn main -> res unit str\n let text try read \"{input}\"\n do print show words text\n ok nil\n"
        );
        let (result, out, err) = run(&source, &[]);
        assert_eq!(result.expect("no trap"), 0, "err: {err}");
        assert_eq!(out, "3\n");
        let _ = fs::remove_dir_all(&dir);
    }
    #[derive(Clone)]
    struct SharedSink(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl Write for SharedSink {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("shared sink lock")
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn stderr_writes_flush_stdout_first() {
        let file = parse_ok(
            "fn main -> res unit str\n do print \"out\"\n do eprint \"err\"\n err \"bad\"\n",
        );
        let shared = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut out = BufWriter::new(SharedSink(shared.clone()));
        let mut err = SharedSink(shared.clone());
        {
            let mut interp = Interp::new(&file, Vec::new(), &mut out, &mut err);
            assert_eq!(interp.interpret_main().expect("no trap"), 1);
        }
        drop(out);
        let observed = String::from_utf8(shared.lock().expect("shared sink lock").clone())
            .expect("utf-8 output");
        assert_eq!(observed, "out\nerr\nbad\n");
    }
}
