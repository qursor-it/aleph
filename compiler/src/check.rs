//! Static type checker (SPEC §4-5).
//!
//! The checker is bidirectional: [`TypeChecker::check_expr`] takes an optional
//! expected type and unifies the inferred type with it at the expression's
//! node. Types are lowered from [`ast::Type`]; unknown named types report
//! `T002` at the type node and become [`Type::Error`], which unifies with
//! every other type without a diagnostic (so one mistake does not cascade).
//!
//! Generic builtins are instantiated with fresh type variables per call (and
//! per `@f` reference). Bounds such as `sort`'s "T is primitive" are checked
//! when the enclosing function has been fully checked, so argument inference
//! has resolved as many variables as possible.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ast::{self, BinOp, CalleeKind, Expr, ExprKind, NodeId, Span, StmtKind, TypeKind, UnOp};
use crate::builtins::{self, Bound, Effect, Ty as BuiltinTy, TyVar};
use crate::diag::Diagnostic;
use crate::lexicon;
use crate::printer;

/// A checked type. `Var` is an inference variable; `Error` is the recovery
/// type produced after a diagnostic and unifies with everything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I64,
    F64,
    Bool,
    Str,
    Unit,
    Json,
    List(Box<Type>),
    Opt(Box<Type>),
    Res(Box<Type>, Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Record(String),
    Fn(Vec<Type>, Box<Type>),
    Var(u32),
    Error,
}

impl fmt::Display for Type {
    /// Canonical spelling (SPEC §4) used by diagnostics and `aleph explain`.
    /// Inference variables print as `_`, function types as `(A B) -> R`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I64 => formatter.write_str(lexicon::type_name("int").expect("i64")),
            Type::F64 => formatter.write_str(lexicon::type_name("float").expect("f64")),
            Type::Bool => formatter.write_str(lexicon::type_name("boolean").expect("bool")),
            Type::Str => formatter.write_str(lexicon::type_name("string").expect("str")),
            Type::Unit => formatter.write_str(lexicon::type_name("unit").expect("unit")),
            Type::Json => formatter.write_str(lexicon::type_name("json").expect("json")),
            Type::List(elem) => write!(
                formatter,
                "{} {}",
                lexicon::type_name("list").expect("list"),
                elem
            ),
            Type::Opt(elem) => write!(
                formatter,
                "{} {}",
                lexicon::type_name("option").expect("opt"),
                elem
            ),
            Type::Res(ok, err) => write!(
                formatter,
                "{} {} {}",
                lexicon::type_name("result").expect("res"),
                ok,
                err
            ),
            Type::Dict(key, val) => write!(
                formatter,
                "{} {} {}",
                lexicon::type_name("dict").expect("dict"),
                key,
                val
            ),
            Type::Record(name) => formatter.write_str(name),
            Type::Fn(params, ret) => {
                let params: Vec<String> = params.iter().map(ToString::to_string).collect();
                write!(formatter, "({}) -> {}", params.join(" "), ret)
            }
            Type::Var(_) => formatter.write_str("_"),
            Type::Error => formatter.write_str("error"),
        }
    }
}

/// The result of checking one file.
#[derive(Debug, Clone, Default)]
pub struct CheckOutput {
    pub diagnostics: Vec<Diagnostic>,
    /// Constructor (record type or sum variant) name to its fields, in
    /// declaration order.
    pub records: BTreeMap<String, Vec<(String, Type)>>,
    /// Function name to its parameter types and return type.
    pub signatures: BTreeMap<String, (Vec<Type>, Type)>,
    /// The resolved type of every expression node, keyed by node id.
    pub expr_types: BTreeMap<NodeId, Type>,
}

/// Check a parsed file (SPEC §4-5).
pub fn check(file: &str, ast: &ast::File) -> CheckOutput {
    TypeChecker::new(file, ast).run()
}

/// A constraint on a type variable, checked when the enclosing function is
/// fully checked.
#[derive(Debug, Clone, Copy)]
enum DeferredKind {
    Builtin(Bound),
    Numeric,
    Primitive,
    I64,
}

/// A bound check postponed until the end of the enclosing function.
struct DeferredBound {
    id: NodeId,
    span: Span,
    ty: Type,
    kind: DeferredKind,
}

struct TypeChecker<'a> {
    file: &'a str,
    ast: &'a ast::File,
    diagnostics: Vec<Diagnostic>,
    records: BTreeMap<String, Vec<(String, Type)>>,
    /// Variant name to its sum type name (ADR-0034).
    variant_sum: BTreeMap<String, String>,
    /// Sum type names.
    sums: BTreeSet<String>,
    signatures: BTreeMap<String, (Vec<Type>, Type)>,
    subst: Vec<Option<Type>>,
    next_var: u32,
    env: BTreeMap<String, Type>,
    current_ret: Option<Type>,
    deferred: Vec<DeferredBound>,
    /// Nodes that already carry a diagnostic, used to avoid a second `T001`.
    reported: BTreeSet<NodeId>,
    expr_types: BTreeMap<NodeId, Type>,
}

impl<'a> TypeChecker<'a> {
    fn new(file: &'a str, ast: &'a ast::File) -> Self {
        Self {
            file,
            ast,
            diagnostics: Vec::new(),
            records: BTreeMap::new(),
            variant_sum: BTreeMap::new(),
            sums: BTreeSet::new(),
            signatures: BTreeMap::new(),
            subst: Vec::new(),
            next_var: 0,
            env: BTreeMap::new(),
            current_ret: None,
            deferred: Vec::new(),
            reported: BTreeSet::new(),
            expr_types: BTreeMap::new(),
        }
    }

    fn run(mut self) -> CheckOutput {
        let ast = self.ast;

        // Type names first, so fields may refer to types defined later.
        for item in &ast.types {
            if item.variants.is_empty() {
                self.records.insert(item.name.clone(), Vec::new());
            } else {
                self.sums.insert(item.name.clone());
                for variant in &item.variants {
                    self.variant_sum
                        .insert(variant.name.clone(), item.name.clone());
                }
            }
        }
        for item in &ast.types {
            let fields = self.lower_fields(&item.fields);
            if item.variants.is_empty() {
                self.records.insert(item.name.clone(), fields);
            }
            for variant in &item.variants {
                let fields = self.lower_fields(&variant.fields);
                self.records.insert(variant.name.clone(), fields);
            }
        }

        for function in &ast.fns {
            let params = function
                .params
                .iter()
                .map(|param| self.lower_type(&param.ty))
                .collect();
            let ret = self.lower_type(&function.ret);
            self.signatures.insert(function.name.clone(), (params, ret));
        }

        for function in &ast.fns {
            self.check_main_signature(function);
        }
        for function in &ast.fns {
            self.check_fn(function);
        }

        self.diagnostics
            .sort_by_key(|diagnostic| (diagnostic.line, diagnostic.col));

        // Bounds are flushed per function, so resolve every recorded type once
        // more with the final substitution before handing it to codegen.
        let expr_types = std::mem::take(&mut self.expr_types)
            .into_iter()
            .map(|(id, ty)| (id, self.resolve(&ty)))
            .collect();

        CheckOutput {
            diagnostics: self.diagnostics,
            records: self.records,
            signatures: self.signatures,
            expr_types,
        }
    }

    // -- Lowering -----------------------------------------------------------

    fn lower_fields(&mut self, fields: &[ast::Field]) -> Vec<(String, Type)> {
        fields
            .iter()
            .map(|field| (field.name.clone(), self.lower_type(&field.ty)))
            .collect()
    }

    /// The type a constructor returns: its sum for a variant, else the record.
    fn ctor_type(&self, name: &str) -> Type {
        Type::Record(
            self.variant_sum
                .get(name)
                .cloned()
                .unwrap_or(name.to_string()),
        )
    }

    fn lower_type(&mut self, ty: &ast::Type) -> Type {
        match &ty.kind {
            TypeKind::I64 => Type::I64,
            TypeKind::F64 => Type::F64,
            TypeKind::Bool => Type::Bool,
            TypeKind::Str => Type::Str,
            TypeKind::Unit => Type::Unit,
            TypeKind::Json => Type::Json,
            TypeKind::List { elem } => Type::List(Box::new(self.lower_type(elem))),
            TypeKind::Opt { elem } => Type::Opt(Box::new(self.lower_type(elem))),
            TypeKind::Res { ok, err } => Type::Res(
                Box::new(self.lower_type(ok)),
                Box::new(self.lower_type(err)),
            ),
            TypeKind::Dict { key, val } => {
                let key_ty = self.lower_type(key);
                // Lowered before the key check so its errors are reported too.
                let val_ty = self.lower_type(val);
                if !matches!(key_ty, Type::I64 | Type::Bool | Type::Str | Type::Error) {
                    self.push_diag_expected_found(
                        key.id,
                        key.span,
                        "T004",
                        "type constraint not satisfied",
                        "key",
                        key_ty.to_string(),
                    );
                    return Type::Error;
                }
                Type::Dict(Box::new(key_ty), Box::new(val_ty))
            }
            TypeKind::Named { name } => {
                let is_record =
                    self.records.contains_key(name) && !self.variant_sum.contains_key(name);
                if is_record || self.sums.contains(name) {
                    Type::Record(name.clone())
                } else {
                    self.push_diag(ty.id, ty.span, "T002", "unknown type");
                    Type::Error
                }
            }
        }
    }

    // -- Inference variables ------------------------------------------------

    fn fresh(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        self.subst.push(None);
        Type::Var(id)
    }

    fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(id) => match self.subst.get(*id as usize).and_then(Option::as_ref) {
                Some(bound) => self.resolve(bound),
                None => Type::Var(*id),
            },
            Type::List(elem) => Type::List(Box::new(self.resolve(elem))),
            Type::Opt(elem) => Type::Opt(Box::new(self.resolve(elem))),
            Type::Res(ok, err) => {
                Type::Res(Box::new(self.resolve(ok)), Box::new(self.resolve(err)))
            }
            Type::Dict(key, val) => {
                Type::Dict(Box::new(self.resolve(key)), Box::new(self.resolve(val)))
            }
            Type::Fn(params, ret) => Type::Fn(
                params.iter().map(|param| self.resolve(param)).collect(),
                Box::new(self.resolve(ret)),
            ),
            other => other.clone(),
        }
    }

    fn occurs(&self, var: u32, ty: &Type) -> bool {
        match self.resolve(ty) {
            Type::Var(id) => id == var,
            Type::List(elem) | Type::Opt(elem) => self.occurs(var, &elem),
            Type::Res(ok, err) | Type::Dict(ok, err) => {
                self.occurs(var, &ok) || self.occurs(var, &err)
            }
            Type::Fn(params, ret) => {
                params.iter().any(|param| self.occurs(var, param)) || self.occurs(var, &ret)
            }
            _ => false,
        }
    }

    /// Unify two types, mutating the substitution. `Error` unifies with
    /// anything silently. A failed unification restores the substitution, so
    /// a probe never leaves partial bindings behind.
    fn unify(&mut self, left: &Type, right: &Type) -> bool {
        let snapshot = self.subst.clone();
        if self.unify_inner(left, right) {
            true
        } else {
            self.subst = snapshot;
            false
        }
    }

    fn unify_inner(&mut self, left: &Type, right: &Type) -> bool {
        let left = self.resolve(left);
        let right = self.resolve(right);
        match (&left, &right) {
            (Type::Error, _) | (_, Type::Error) => true,
            (Type::Var(a), Type::Var(b)) if a == b => true,
            (Type::Var(a), _) => {
                if self.occurs(*a, &right) {
                    false
                } else {
                    self.subst[*a as usize] = Some(right);
                    true
                }
            }
            (_, Type::Var(b)) => {
                if self.occurs(*b, &left) {
                    false
                } else {
                    self.subst[*b as usize] = Some(left);
                    true
                }
            }
            (Type::I64, Type::I64)
            | (Type::F64, Type::F64)
            | (Type::Bool, Type::Bool)
            | (Type::Str, Type::Str)
            | (Type::Unit, Type::Unit)
            | (Type::Json, Type::Json) => true,
            (Type::Record(a), Type::Record(b)) => a == b,
            (Type::List(a), Type::List(b)) | (Type::Opt(a), Type::Opt(b)) => self.unify_inner(a, b),
            (Type::Res(a_ok, a_err), Type::Res(b_ok, b_err))
            | (Type::Dict(a_ok, a_err), Type::Dict(b_ok, b_err)) => {
                self.unify_inner(a_ok, b_ok) && self.unify_inner(a_err, b_err)
            }
            (Type::Fn(a_params, a_ret), Type::Fn(b_params, b_ret)) => {
                a_params.len() == b_params.len()
                    && a_params
                        .iter()
                        .zip(b_params)
                        .all(|(a, b)| self.unify_inner(a, b))
                    && self.unify_inner(a_ret, b_ret)
            }
            _ => false,
        }
    }

    // -- Functions and statements ------------------------------------------

    fn check_main_signature(&mut self, function: &ast::Fn) {
        if function.name != "main" {
            return;
        }
        let Some((params, ret)) = self.signatures.get("main").cloned() else {
            return;
        };
        if params.iter().any(contains_error) || contains_error(&ret) {
            return;
        }
        let valid_ret = ret == Type::Unit
            || matches!(&ret, Type::Res(ok, err) if **ok == Type::Unit && **err == Type::Str);
        if !params.is_empty() || !valid_ret {
            self.push_diag(function.id, function.span, "T008", "invalid main signature");
        }
    }

    fn check_fn(&mut self, function: &ast::Fn) {
        self.deferred.clear();
        self.env.clear();
        self.current_ret = self
            .signatures
            .get(&function.name)
            .map(|(_, ret)| ret.clone());

        // Examples see globals only (v0.1 has no global values).
        for example in &function.examples {
            self.env.clear();
            let lhs = self.check_expr(&example.lhs, None);
            let lhs = self.resolve(&lhs);
            if contains_error(&lhs) {
                self.check_expr(&example.rhs, None);
            } else {
                self.check_expr(&example.rhs, Some(&lhs));
            }
        }

        self.env.clear();
        if let Some((params, _)) = self.signatures.get(&function.name).cloned() {
            for (param, ty) in function.params.iter().zip(params) {
                self.env.insert(param.name.clone(), ty);
            }
        }

        for stmt in &function.stmts {
            match &stmt.kind {
                StmtKind::Let { name, value } => {
                    let ty = self.check_expr(value, None);
                    self.env.insert(name.clone(), ty);
                }
                StmtKind::Do { value } => {
                    self.check_expr(value, Some(&Type::Unit));
                }
            }
        }

        let ret = self.current_ret.clone().unwrap_or(Type::Unit);
        self.check_expr(&function.result, Some(&ret));
        self.flush_bounds();
    }

    fn flush_bounds(&mut self) {
        let deferred = std::mem::take(&mut self.deferred);
        for item in deferred {
            let ty = self.resolve(&item.ty);
            if matches!(ty, Type::Var(_)) || contains_error(&ty) {
                continue;
            }
            let satisfied = match item.kind {
                DeferredKind::Builtin(bound) => satisfies(&ty, bound),
                DeferredKind::Numeric => is_numeric(&ty),
                DeferredKind::Primitive => is_primitive(&ty),
                DeferredKind::I64 => ty == Type::I64,
            };
            if !satisfied {
                let expected = deferred_text(item.kind);
                let found = ty.to_string();
                self.push_diag_expected_found(
                    item.id,
                    item.span,
                    "T004",
                    "type constraint not satisfied",
                    expected,
                    found,
                );
            }
        }
    }

    // -- Expressions --------------------------------------------------------

    fn check_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        let ty = self.check_expr_inner(expr, expected);
        let ty = self.resolve(&ty);
        self.expr_types.insert(expr.id, ty.clone());
        ty
    }

    fn check_expr_inner(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        let inferred = self.infer(expr, expected);
        if let Some(expected) = expected {
            let expected = self.resolve(expected);
            let found = self.resolve(&inferred);
            if contains_error(&found) || contains_error(&expected) {
                return self.resolve(&inferred);
            }
            if !self.unify(&found, &expected) {
                self.report_t001(expr, &expected, &found);
                return Type::Error;
            }
            self.resolve(&inferred)
        } else {
            self.resolve(&inferred)
        }
    }

    fn infer(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        match &expr.kind {
            ExprKind::Int { .. } => Type::I64,
            ExprKind::Float { .. } => Type::F64,
            ExprKind::Str { .. } => Type::Str,
            ExprKind::Bool { .. } => Type::Bool,
            ExprKind::Nil => Type::Unit,
            ExprKind::Var { name } => self.env.get(name).cloned().unwrap_or(Type::Error),
            ExprKind::FnRef { name } => self.check_fn_ref(expr, name, expected),
            ExprKind::Call {
                callee,
                callee_kind,
                args,
            } => self.check_call(expr, callee, *callee_kind, args),
            ExprKind::BinOp { op, lhs, rhs } => self.check_binop(expr, *op, lhs, rhs),
            ExprKind::UnOp { op, operand } => self.check_unop(expr, *op, operand),
            ExprKind::If { cond, then_, else_ } => {
                self.check_cond(cond);
                let then_ty = self.check_expr(then_, expected);
                let then_ty = self.resolve(&then_ty);
                if contains_error(&then_ty) {
                    self.check_expr(else_, expected);
                } else {
                    self.check_expr(else_, Some(&then_ty));
                }
                then_ty
            }
            ExprKind::List { elems } => self.check_list(elems, expected),
            ExprKind::Empty { ty } => Type::List(Box::new(self.lower_type(ty))),
            ExprKind::Dict { ty } => self.lower_type(ty),
            ExprKind::Field { name, value } => self.check_field(expr, name, value),
            ExprKind::Some { value } => self.check_some(value, expected),
            ExprKind::None { ty } => Type::Opt(Box::new(self.lower_type(ty))),
            ExprKind::Ok { value } => self.check_ok(expr, value, expected),
            ExprKind::Err { value } => self.check_err(expr, value, expected),
            ExprKind::Try { value } => self.check_try(expr, value),
            ExprKind::Else { value, default } => self.check_else(expr, value, default, expected),
            ExprKind::Has { value } => self.check_has(value),
            ExprKind::IsOk { value } => self.check_isok(value),
            ExprKind::Match { value, arms } => self.check_match(value, arms, expected),
        }
    }

    /// `match`: the scrutinee has the arms' sum type; the expected type flows
    /// into the first arm, whose type then flows into the others (as `if`).
    fn check_match(&mut self, value: &Expr, arms: &[ast::Arm], expected: Option<&Type>) -> Type {
        let sum = arms
            .first()
            .map(|arm| self.ctor_type(&arm.variant))
            .unwrap_or(Type::Error);
        self.check_expr(value, Some(&sum));
        let mut first: Option<Type> = None;
        for arm in arms {
            let fields = self.records.get(&arm.variant).cloned().unwrap_or_default();
            let saved = self.env.clone();
            for (binder, (_, ty)) in arm.binders.iter().zip(fields) {
                self.env.insert(binder.clone(), ty);
            }
            let want = match &first {
                Some(ty) if !contains_error(ty) => Some(ty.clone()),
                _ => expected.cloned(),
            };
            let ty = self.check_expr(&arm.body, want.as_ref());
            self.env = saved;
            if first.is_none() {
                first = Some(ty);
            }
        }
        first.unwrap_or(Type::Error)
    }

    fn check_cond(&mut self, cond: &Expr) {
        self.check_expr(cond, Some(&Type::Bool));
    }

    fn check_fn_ref(&mut self, expr: &Expr, name: &str, expected: Option<&Type>) -> Type {
        let expected_fn = expected.and_then(|ty| match self.resolve(ty) {
            Type::Fn(params, ret) => Some((params, *ret)),
            _ => None,
        });
        let Some(_) = expected_fn else {
            self.push_diag(
                expr.id,
                expr.span,
                "T006",
                "function reference not allowed here",
            );
            return Type::Error;
        };

        if let Some((params, ret)) = self.signatures.get(name).cloned() {
            return Type::Fn(params, Box::new(ret));
        }
        if let Some(fields) = self.records.get(name).cloned() {
            let params = fields.into_iter().map(|(_, ty)| ty).collect();
            return Type::Fn(params, Box::new(self.ctor_type(name)));
        }
        if let Some(spec) = builtins::lookup(name) {
            if spec.effect == Effect::Effectful {
                self.push_diag(
                    expr.id,
                    expr.span,
                    "T006",
                    "function reference not allowed here",
                );
                return Type::Error;
            }
            let mut slots = [self.fresh(), self.fresh(), self.fresh()];
            let params = spec
                .params
                .iter()
                .map(|param| self.instantiate(param, &mut slots))
                .collect();
            let ret = self.instantiate(spec.ret, &mut slots);
            for (var, bound) in spec.bounds {
                self.defer_bound(expr, &slots[var_index(*var)], DeferredKind::Builtin(*bound));
            }
            return Type::Fn(params, Box::new(ret));
        }
        Type::Error
    }

    fn check_call(&mut self, expr: &Expr, callee: &str, kind: CalleeKind, args: &[Expr]) -> Type {
        match kind {
            CalleeKind::Fn => {
                let Some((params, ret)) = self.signatures.get(callee).cloned() else {
                    return Type::Error;
                };
                for (arg, param) in args.iter().zip(&params) {
                    self.check_expr(arg, Some(param));
                }
                ret
            }
            CalleeKind::Ctor => {
                let Some(fields) = self.records.get(callee).cloned() else {
                    return Type::Error;
                };
                for (arg, (_, ty)) in args.iter().zip(&fields) {
                    self.check_expr(arg, Some(ty));
                }
                self.ctor_type(callee)
            }
            CalleeKind::Builtin => {
                let Some(spec) = builtins::lookup(callee) else {
                    return Type::Error;
                };
                let mut slots = [self.fresh(), self.fresh(), self.fresh()];
                let params: Vec<Type> = spec
                    .params
                    .iter()
                    .map(|param| self.instantiate(param, &mut slots))
                    .collect();
                let ret = self.instantiate(spec.ret, &mut slots);
                for (arg, param) in args.iter().zip(&params) {
                    self.check_expr(arg, Some(param));
                }
                for (var, bound) in spec.bounds {
                    self.defer_bound(expr, &slots[var_index(*var)], DeferredKind::Builtin(*bound));
                }
                ret
            }
        }
    }

    fn instantiate(&mut self, ty: &BuiltinTy, slots: &mut [Type; 3]) -> Type {
        match ty {
            BuiltinTy::I64 => Type::I64,
            BuiltinTy::F64 => Type::F64,
            BuiltinTy::Bool => Type::Bool,
            BuiltinTy::Str => Type::Str,
            BuiltinTy::Unit => Type::Unit,
            BuiltinTy::Json => Type::Json,
            BuiltinTy::List(elem) => Type::List(Box::new(self.instantiate(elem, slots))),
            BuiltinTy::Opt(elem) => Type::Opt(Box::new(self.instantiate(elem, slots))),
            BuiltinTy::Res(ok, err) => Type::Res(
                Box::new(self.instantiate(ok, slots)),
                Box::new(self.instantiate(err, slots)),
            ),
            BuiltinTy::Dict(key, val) => Type::Dict(
                Box::new(self.instantiate(key, slots)),
                Box::new(self.instantiate(val, slots)),
            ),
            BuiltinTy::Var(var) => slots[var_index(*var)].clone(),
            BuiltinTy::Fn(params, ret) => Type::Fn(
                params
                    .iter()
                    .map(|param| self.instantiate(param, slots))
                    .collect(),
                Box::new(self.instantiate(ret, slots)),
            ),
        }
    }

    fn defer_bound(&mut self, expr: &Expr, ty: &Type, kind: DeferredKind) {
        self.deferred.push(DeferredBound {
            id: expr.id,
            span: expr.span,
            ty: ty.clone(),
            kind,
        });
    }

    fn check_binop(&mut self, expr: &Expr, op: BinOp, lhs: &Expr, rhs: &Expr) -> Type {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => self.check_arith(expr, lhs, rhs),
            BinOp::Mod => self.check_mod(expr, lhs, rhs),
            BinOp::Eq | BinOp::Ne => self.check_eq(expr, lhs, rhs),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                if self.check_arith(expr, lhs, rhs) == Type::Error {
                    Type::Error
                } else {
                    Type::Bool
                }
            }
            BinOp::And | BinOp::Or => {
                self.check_expr(lhs, Some(&Type::Bool));
                self.check_expr(rhs, Some(&Type::Bool));
                Type::Bool
            }
        }
    }

    /// Infer both operands and let a lone inference variable take the other
    /// operand's type. Two unresolved variables are unified with each other;
    /// the caller then defers the operator's constraint on that variable.
    fn check_operands(&mut self, lhs: &Expr, rhs: &Expr) -> (Type, Type) {
        let left = self.check_expr(lhs, None);
        let left = self.resolve(&left);
        let right = self.check_expr(rhs, None);
        let right = self.resolve(&right);
        if !contains_error(&left)
            && !contains_error(&right)
            && (matches!(left, Type::Var(_)) || matches!(right, Type::Var(_)))
        {
            let _ = self.unify(&left, &right);
        }
        (self.resolve(&left), self.resolve(&right))
    }

    fn check_arith(&mut self, expr: &Expr, lhs: &Expr, rhs: &Expr) -> Type {
        let (left, right) = self.check_operands(lhs, rhs);
        if contains_error(&left) || contains_error(&right) {
            return Type::Error;
        }
        if matches!((&left, &right), (Type::Var(_), Type::Var(_))) {
            self.defer_bound(expr, &left, DeferredKind::Numeric);
            return left;
        }
        if !is_numeric(&left) {
            let found = left.to_string();
            self.push_diag_expected_found(
                lhs.id,
                lhs.span,
                "T001",
                "type mismatch",
                "i64 or f64",
                found,
            );
            return Type::Error;
        }
        if !is_numeric(&right) {
            let found = right.to_string();
            self.push_diag_expected_found(
                rhs.id,
                rhs.span,
                "T001",
                "type mismatch",
                "i64 or f64",
                found,
            );
            return Type::Error;
        }
        if left != right {
            let expected = left.to_string();
            let found = right.to_string();
            self.push_diag_expected_found(
                rhs.id,
                rhs.span,
                "T001",
                "type mismatch",
                expected,
                found,
            );
            return Type::Error;
        }
        left
    }

    fn check_mod(&mut self, expr: &Expr, lhs: &Expr, rhs: &Expr) -> Type {
        let (left, right) = self.check_operands(lhs, rhs);
        if contains_error(&left) || contains_error(&right) {
            return Type::Error;
        }
        if matches!((&left, &right), (Type::Var(_), Type::Var(_))) {
            self.defer_bound(expr, &left, DeferredKind::I64);
            return Type::I64;
        }
        if left != Type::I64 {
            let found = left.to_string();
            self.push_diag_expected_found(lhs.id, lhs.span, "T001", "type mismatch", "i64", found);
            return Type::Error;
        }
        if right != Type::I64 {
            let found = right.to_string();
            self.push_diag_expected_found(rhs.id, rhs.span, "T001", "type mismatch", "i64", found);
            return Type::Error;
        }
        Type::I64
    }

    fn check_eq(&mut self, expr: &Expr, lhs: &Expr, rhs: &Expr) -> Type {
        let (left, right) = self.check_operands(lhs, rhs);
        if contains_error(&left) || contains_error(&right) {
            return Type::Error;
        }
        if matches!((&left, &right), (Type::Var(_), Type::Var(_))) {
            self.defer_bound(expr, &left, DeferredKind::Primitive);
            return Type::Bool;
        }
        if !is_primitive(&left) {
            let found = left.to_string();
            self.push_diag_expected_found(
                lhs.id,
                lhs.span,
                "T004",
                "type constraint not satisfied",
                "primitive",
                found,
            );
            return Type::Error;
        }
        if !is_primitive(&right) {
            let found = right.to_string();
            self.push_diag_expected_found(
                rhs.id,
                rhs.span,
                "T004",
                "type constraint not satisfied",
                "primitive",
                found,
            );
            return Type::Error;
        }
        if left != right {
            let expected = left.to_string();
            let found = right.to_string();
            self.push_diag_expected_found(
                rhs.id,
                rhs.span,
                "T001",
                "type mismatch",
                expected,
                found,
            );
            return Type::Error;
        }
        Type::Bool
    }

    fn check_unop(&mut self, expr: &Expr, op: UnOp, operand: &Expr) -> Type {
        match op {
            UnOp::Not => {
                self.check_expr(operand, Some(&Type::Bool));
                Type::Bool
            }
            UnOp::Neg => {
                let ty = self.check_expr(operand, None);
                let ty = self.resolve(&ty);
                if contains_error(&ty) {
                    return Type::Error;
                }
                if matches!(ty, Type::Var(_)) {
                    self.defer_bound(expr, &ty, DeferredKind::Numeric);
                    return ty;
                }
                if is_numeric(&ty) {
                    ty
                } else {
                    let found = ty.to_string();
                    self.push_diag_expected_found(
                        operand.id,
                        operand.span,
                        "T001",
                        "type mismatch",
                        "i64 or f64",
                        found,
                    );
                    Type::Error
                }
            }
        }
    }

    fn check_list(&mut self, elems: &[Expr], expected: Option<&Type>) -> Type {
        let expected_elem = expected.and_then(|ty| match self.resolve(ty) {
            Type::List(elem) => Some(*elem),
            _ => None,
        });
        if elems.is_empty() {
            return Type::List(Box::new(expected_elem.unwrap_or(Type::Error)));
        }
        let elem = match expected_elem {
            Some(elem) => {
                for expression in elems {
                    self.check_expr(expression, Some(&elem));
                }
                self.resolve(&elem)
            }
            None => {
                let first = self.check_expr(&elems[0], None);
                let first = self.resolve(&first);
                for expression in &elems[1..] {
                    self.check_expr(expression, Some(&first));
                }
                first
            }
        };
        Type::List(Box::new(elem))
    }

    fn check_field(&mut self, expr: &Expr, name: &str, value: &Expr) -> Type {
        let base = self.check_expr(value, None);
        let base = self.resolve(&base);
        match base {
            Type::Record(record) => {
                let field = self
                    .records
                    .get(&record)
                    .and_then(|fields| fields.iter().find(|(field, _)| field == name))
                    .map(|(_, ty)| ty.clone());
                match field {
                    Some(ty) => ty,
                    None => {
                        self.push_diag_expected_found(
                            expr.id,
                            expr.span,
                            "T003",
                            "invalid field access",
                            record,
                            name.to_string(),
                        );
                        Type::Error
                    }
                }
            }
            other if contains_error(&other) => Type::Error,
            other => {
                let found = other.to_string();
                self.push_diag_expected_found(
                    expr.id,
                    expr.span,
                    "T003",
                    "invalid field access",
                    "record",
                    found,
                );
                Type::Error
            }
        }
    }

    fn check_some(&mut self, value: &Expr, expected: Option<&Type>) -> Type {
        let expected_elem = expected.and_then(|ty| match self.resolve(ty) {
            Type::Opt(elem) => Some(*elem),
            _ => None,
        });
        match expected_elem {
            Some(elem) => {
                self.check_expr(value, Some(&elem));
                Type::Opt(Box::new(self.resolve(&elem)))
            }
            None => Type::Opt(Box::new(self.check_expr(value, None))),
        }
    }

    fn check_ok(&mut self, expr: &Expr, value: &Expr, expected: Option<&Type>) -> Type {
        let expected = expected.map(|ty| self.resolve(ty));
        if let Some(ty) = &expected {
            if contains_error(ty) {
                self.check_expr(value, None);
                return Type::Error;
            }
        }
        match expected {
            Some(Type::Res(ok, err)) => {
                let checked = self.check_expr(value, Some(&ok));
                if contains_error(&checked) {
                    Type::Error
                } else {
                    Type::Res(ok, err)
                }
            }
            Some(Type::Var(_)) | None => {
                self.push_diag(
                    expr.id,
                    expr.span,
                    "T010",
                    "cannot infer type of ok/err/none",
                );
                self.check_expr(value, None);
                Type::Error
            }
            Some(other) => {
                let expected = other.to_string();
                self.push_diag_expected_found(
                    expr.id,
                    expr.span,
                    "T001",
                    "type mismatch",
                    expected,
                    "res _ _",
                );
                self.check_expr(value, None);
                Type::Error
            }
        }
    }

    fn check_err(&mut self, expr: &Expr, value: &Expr, expected: Option<&Type>) -> Type {
        let expected = expected.map(|ty| self.resolve(ty));
        if let Some(ty) = &expected {
            if contains_error(ty) {
                self.check_expr(value, None);
                return Type::Error;
            }
        }
        match expected {
            Some(Type::Res(ok, err)) => {
                let checked = self.check_expr(value, Some(&err));
                if contains_error(&checked) {
                    Type::Error
                } else {
                    Type::Res(ok, err)
                }
            }
            Some(Type::Var(_)) | None => {
                self.push_diag(
                    expr.id,
                    expr.span,
                    "T010",
                    "cannot infer type of ok/err/none",
                );
                self.check_expr(value, None);
                Type::Error
            }
            Some(other) => {
                let expected = other.to_string();
                self.push_diag_expected_found(
                    expr.id,
                    expr.span,
                    "T001",
                    "type mismatch",
                    expected,
                    "res _ _",
                );
                self.check_expr(value, None);
                Type::Error
            }
        }
    }

    fn check_try(&mut self, expr: &Expr, value: &Expr) -> Type {
        let ret = self.current_ret.clone().unwrap_or(Type::Unit);
        let ret = self.resolve(&ret);
        if contains_error(&ret) {
            self.check_expr(value, None);
            return Type::Error;
        }
        match ret {
            Type::Res(_, err) => {
                let ok = self.fresh();
                let checked = self.check_expr(value, Some(&Type::Res(Box::new(ok.clone()), err)));
                if contains_error(&checked) {
                    Type::Error
                } else {
                    ok
                }
            }
            _ => {
                self.push_diag(expr.id, expr.span, "T007", "try outside result function");
                self.check_expr(value, None);
                Type::Error
            }
        }
    }

    fn check_else(
        &mut self,
        expr: &Expr,
        value: &Expr,
        default: &Expr,
        expected: Option<&Type>,
    ) -> Type {
        // The default branch is the result type: check it against the expected
        // type first, or infer it when there is no context.
        let result = match expected {
            Some(expected) => {
                let expected = self.resolve(expected);
                self.check_expr(default, Some(&expected));
                expected
            }
            None => {
                let inferred = self.check_expr(default, None);
                self.resolve(&inferred)
            }
        };
        if contains_error(&result) {
            self.check_expr(value, None);
            return Type::Error;
        }

        match value_shape(value) {
            ValueShape::Opt => {
                let container = Type::Opt(Box::new(result.clone()));
                let checked = self.check_expr(value, Some(&container));
                if contains_error(&checked) {
                    Type::Error
                } else {
                    result
                }
            }
            ValueShape::Res => {
                let container = Type::Res(Box::new(result.clone()), Box::new(self.fresh()));
                let checked = self.check_expr(value, Some(&container));
                if contains_error(&checked) {
                    Type::Error
                } else {
                    result
                }
            }
            ValueShape::Unknown => {
                let value_ty = self.check_expr(value, None);
                let value_ty = self.resolve(&value_ty);
                if contains_error(&value_ty) {
                    return Type::Error;
                }
                match value_ty {
                    Type::Opt(inner) | Type::Res(inner, _) => {
                        let inner = self.resolve(&inner);
                        if self.unify(&inner, &result) {
                            result
                        } else {
                            let expected = result.to_string();
                            let found = inner.to_string();
                            self.push_diag_expected_found(
                                value.id,
                                value.span,
                                "T001",
                                "type mismatch",
                                expected,
                                found,
                            );
                            Type::Error
                        }
                    }
                    other => {
                        let found = other.to_string();
                        self.push_diag_expected_found(
                            expr.id,
                            expr.span,
                            "T001",
                            "type mismatch",
                            "opt or res",
                            found,
                        );
                        Type::Error
                    }
                }
            }
        }
    }

    fn check_has(&mut self, value: &Expr) -> Type {
        let ty = self.check_expr(value, None);
        let ty = self.resolve(&ty);
        if matches!(ty, Type::Opt(_)) || contains_error(&ty) {
            return Type::Bool;
        }
        let found = ty.to_string();
        self.push_diag_expected_found(
            value.id,
            value.span,
            "T001",
            "type mismatch",
            "opt _",
            found,
        );
        Type::Bool
    }

    fn check_isok(&mut self, value: &Expr) -> Type {
        let ty = self.check_expr(value, None);
        let ty = self.resolve(&ty);
        if matches!(ty, Type::Res(_, _)) || contains_error(&ty) {
            return Type::Bool;
        }
        let found = ty.to_string();
        self.push_diag_expected_found(
            value.id,
            value.span,
            "T001",
            "type mismatch",
            "res _ _",
            found,
        );
        Type::Bool
    }

    // -- Diagnostics --------------------------------------------------------

    fn report_t001(&mut self, expr: &Expr, expected: &Type, found: &Type) {
        if self.reported.contains(&expr.id) {
            return;
        }
        let current_ret = self.current_ret.as_ref().map(|ty| self.resolve(ty));
        let fix = t001_fix(expected, found, expr, current_ret.as_ref());
        self.reported.insert(expr.id);
        let mut diagnostic = Diagnostic::error(
            "T001",
            self.file,
            expr.span.line,
            expr.span.col,
            "type mismatch",
        )
        .with_node(format!("n{}", expr.id.0))
        .with_expected_found(expected.to_string(), found.to_string());
        if let Some(with) = fix {
            diagnostic = diagnostic.with_fix("replace", format!("n{}", expr.id.0), with);
        }
        self.diagnostics.push(diagnostic);
    }

    fn push_diag(&mut self, id: NodeId, span: Span, code: &str, msg: &str) {
        self.reported.insert(id);
        let diagnostic = Diagnostic::error(code, self.file, span.line, span.col, msg)
            .with_node(format!("n{}", id.0));
        self.diagnostics.push(diagnostic);
    }

    fn push_diag_expected_found(
        &mut self,
        id: NodeId,
        span: Span,
        code: &str,
        msg: &str,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) {
        self.reported.insert(id);
        let diagnostic = Diagnostic::error(code, self.file, span.line, span.col, msg)
            .with_node(format!("n{}", id.0))
            .with_expected_found(expected.into(), found.into());
        self.diagnostics.push(diagnostic);
    }
}

/// A certain `T001` fix (ADR-0028), or `None` when the wrapped result is not
/// guaranteed to type-check.
fn t001_fix(
    expected: &Type,
    found: &Type,
    expr: &Expr,
    current_ret: Option<&Type>,
) -> Option<String> {
    if !is_concrete(expected) || !is_concrete(found) {
        return None;
    }
    let text = printer::print_expr(expr);
    let wrap = |keyword: &str| format!("{keyword} {text}");

    match (expected, found) {
        // A result expression where either side matches the found type.
        (Type::Res(ok, err), found) if found == &**ok && ok != err => {
            Some(wrap(lexicon::keyword("resok").expect("ok")))
        }
        (Type::Res(ok, err), found) if found == &**err && ok != err => {
            Some(wrap(lexicon::keyword("reserr").expect("err")))
        }
        // An option expression where the payload matches the found type.
        (Type::Opt(elem), found) if found == &**elem => {
            Some(wrap(lexicon::keyword("optsome").expect("some")))
        }
        // Boolean tests: the found value is an option or result. Not with a
        // bool payload: then the author may mean the payload, not presence.
        (Type::Bool, Type::Opt(elem)) if **elem != Type::Bool => {
            Some(wrap(lexicon::keyword("isome").expect("has")))
        }
        (Type::Bool, Type::Res(ok, _)) if **ok != Type::Bool => {
            Some(wrap(lexicon::keyword("isokay").expect("isok")))
        }
        // `try` is only certain when the enclosing function returns the same
        // error type as the found result.
        (expected, Type::Res(ok, err)) if expected == &**ok && expected != &Type::Bool => {
            match current_ret? {
                Type::Res(_, ret_err) if ret_err == err => {
                    Some(wrap(lexicon::keyword("propagate").expect("try")))
                }
                _ => None,
            }
        }
        // `show` accepts primitives and lists of primitives, except str.
        (Type::Str, found) if !matches!(found, Type::Str) && satisfies(found, Bound::Showable) => {
            Some(format!("show {text}"))
        }
        _ => None,
    }
}

/// Whether a type contains no inference variable or error type.
fn is_concrete(ty: &Type) -> bool {
    match ty {
        Type::Var(_) | Type::Error => false,
        Type::List(elem) | Type::Opt(elem) => is_concrete(elem),
        Type::Res(ok, err) | Type::Dict(ok, err) => is_concrete(ok) && is_concrete(err),
        Type::Fn(params, ret) => params.iter().all(is_concrete) && is_concrete(ret),
        _ => true,
    }
}

fn var_index(var: TyVar) -> usize {
    match var {
        TyVar::T => 0,
        TyVar::A => 1,
        TyVar::B => 2,
    }
}

fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::I64 | Type::F64)
}

fn is_primitive(ty: &Type) -> bool {
    matches!(ty, Type::I64 | Type::F64 | Type::Bool | Type::Str)
}

fn satisfies(ty: &Type, bound: Bound) -> bool {
    match bound {
        Bound::Primitive => is_primitive(ty),
        Bound::Key => matches!(ty, Type::I64 | Type::Bool | Type::Str),
        Bound::Showable => {
            is_primitive(ty)
                || match ty {
                    Type::List(elem) => is_primitive(elem),
                    Type::Json => true,
                    _ => false,
                }
        }
        Bound::Jsonable => match ty {
            Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Json => true,
            Type::List(elem) => **elem == Type::Json,
            Type::Dict(key, val) => **key == Type::Str && **val == Type::Json,
            _ => false,
        },
    }
}

fn deferred_text(kind: DeferredKind) -> String {
    match kind {
        DeferredKind::Builtin(Bound::Primitive) | DeferredKind::Primitive => {
            "primitive".to_string()
        }
        DeferredKind::Builtin(Bound::Showable) => "showable".to_string(),
        DeferredKind::Builtin(Bound::Key) => "key".to_string(),
        DeferredKind::Builtin(Bound::Jsonable) => "jsonable".to_string(),
        DeferredKind::Numeric => "numeric".to_string(),
        DeferredKind::I64 => "i64".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueShape {
    Opt,
    Res,
    Unknown,
}

fn value_shape(expr: &Expr) -> ValueShape {
    match &expr.kind {
        ExprKind::Some { .. } | ExprKind::None { .. } => ValueShape::Opt,
        ExprKind::Ok { .. } | ExprKind::Err { .. } => ValueShape::Res,
        _ => ValueShape::Unknown,
    }
}

fn contains_error(ty: &Type) -> bool {
    match ty {
        Type::Error => true,
        Type::List(elem) | Type::Opt(elem) => contains_error(elem),
        Type::Res(ok, err) | Type::Dict(ok, err) => contains_error(ok) || contains_error(err),
        Type::Fn(params, ret) => params.iter().any(contains_error) || contains_error(ret),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn parse_source(source: &str) -> ast::File {
        let output = parser::parse("t.aph", source);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}\n{source}",
            output.diagnostics
        );
        output.file.expect("parsed file")
    }

    fn check_source(source: &str) -> CheckOutput {
        let file = parse_source(source);
        check("t.aph", &file)
    }

    fn codes(output: &CheckOutput) -> Vec<&str> {
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    fn int_expr() -> Expr {
        Expr {
            id: NodeId(7),
            span: Span { line: 1, col: 1 },
            kind: ExprKind::Int { value: 1 },
        }
    }

    fn t001(expected: &Type, found: &Type, ret: Option<&Type>) -> Option<String> {
        t001_fix(expected, found, &int_expr(), ret)
    }

    #[test]
    fn t001_fixes_cover_the_declared_rules() {
        let res = Type::Res(Box::new(Type::I64), Box::new(Type::Str));
        assert_eq!(
            t001(&res, &Type::I64, None).as_deref(),
            Some("ok 1"),
            "expected res, found ok side"
        );
        assert_eq!(
            t001(&res, &Type::Str, None).as_deref(),
            Some("err 1"),
            "expected res, found error side"
        );
        assert_eq!(
            t001(&Type::Opt(Box::new(Type::I64)), &Type::I64, None).as_deref(),
            Some("some 1")
        );
        assert_eq!(
            t001(&Type::Bool, &Type::Opt(Box::new(Type::I64)), None).as_deref(),
            Some("has 1")
        );
        assert_eq!(t001(&Type::Bool, &res, None).as_deref(), Some("isok 1"));
        assert_eq!(
            t001(&Type::I64, &res, Some(&res)).as_deref(),
            Some("try 1"),
            "enclosing fn returns the same error type"
        );
        assert_eq!(
            t001(&Type::Str, &Type::I64, None).as_deref(),
            Some("show 1")
        );
        assert_eq!(
            t001(&Type::Str, &Type::List(Box::new(Type::Bool)), None).as_deref(),
            Some("show 1"),
            "lists of primitives are showable"
        );
    }

    #[test]
    fn t001_fixes_are_omitted_when_not_certain() {
        let res = Type::Res(Box::new(Type::I64), Box::new(Type::Str));
        // Unresolved expected or found types.
        assert_eq!(t001(&Type::Var(0), &Type::I64, None), None);
        assert_eq!(
            t001(&Type::List(Box::new(Type::Var(0))), &Type::I64, None),
            None
        );
        assert_eq!(t001(&Type::I64, &Type::Var(0), None), None);
        assert_eq!(t001(&Type::I64, &res, Some(&Type::Var(0))), None);
        // Ambiguous result sides.
        let same = Type::Res(Box::new(Type::I64), Box::new(Type::I64));
        assert_eq!(t001(&same, &Type::I64, None), None);
        // try requires an enclosing result with the same error type.
        let other = Type::Res(Box::new(Type::Unit), Box::new(Type::Bool));
        assert_eq!(t001(&Type::I64, &res, Some(&other)), None);
        assert_eq!(t001(&Type::I64, &res, Some(&Type::Unit)), None);
        assert_eq!(
            t001(&Type::Bool, &res, Some(&Type::Unit)).as_deref(),
            Some("isok 1"),
            "rule 6 applies before the try rule"
        );
        // A bool payload makes has/isok ambiguous (presence vs value).
        let opt_bool = Type::Opt(Box::new(Type::Bool));
        let res_bool = Type::Res(Box::new(Type::Bool), Box::new(Type::Str));
        assert_eq!(t001(&Type::Bool, &opt_bool, None), None);
        assert_eq!(t001(&Type::Bool, &res_bool, Some(&res)), None);
        // show does not apply to str or non-showable types.
        assert_eq!(t001(&Type::Str, &Type::Str, None), None);
        assert_eq!(
            t001(
                &Type::Str,
                &Type::List(Box::new(Type::List(Box::new(Type::I64)))),
                None
            ),
            None
        );
        // Unrelated mismatches.
        assert_eq!(t001(&Type::I64, &Type::Bool, None), None);
    }

    #[test]
    fn t001_diagnostics_carry_their_fix() {
        let output = check_source("fn f -> res i64 str\n ex f = ok 1\n 1\n");
        assert_eq!(codes(&output), ["T001"]);
        let diagnostic = &output.diagnostics[0];
        assert_eq!(diagnostic.node.as_deref(), Some("n9"));
        let fix = diagnostic.fix.as_ref().expect("certain fix");
        assert_eq!(fix.op, "replace");
        assert_eq!(fix.node, "n9");
        assert_eq!(fix.with, "ok 1");
    }

    #[test]
    fn unify_binds_vars_and_rejects_mismatches() {
        let file = parse_source("fn main -> unit\n nil\n");
        let mut checker = TypeChecker::new("t.aph", &file);
        let var = checker.fresh();
        assert!(checker.unify(&var, &Type::I64));
        assert_eq!(checker.resolve(&var), Type::I64);
        assert!(!checker.unify(&Type::I64, &Type::Str));

        let var = checker.fresh();
        let list = Type::List(Box::new(var.clone()));
        assert!(!checker.unify(&var, &list), "occurs check");

        let var = checker.fresh();
        let left = Type::Res(Box::new(var.clone()), Box::new(Type::Bool));
        let right = Type::Res(Box::new(Type::I64), Box::new(Type::Str));
        assert!(!checker.unify(&left, &right));
        assert_eq!(checker.resolve(&var), var, "failed unify must roll back");
    }

    #[test]
    fn error_type_unifies_silently_and_does_not_cascade() {
        let output = check_source("fn f r:res i64 str -> i64\n let x try r\n x\n");
        assert_eq!(codes(&output), ["T007"]);
    }

    #[test]
    fn each_call_gets_fresh_type_variables() {
        let output = check_source("fn f -> i64\n + len [1] len [\"a\"]\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn sort_and_show_bounds_are_checked() {
        let output = check_source("fn f -> list list i64\n sort [[1]]\n");
        assert_eq!(codes(&output), ["T004"]);

        let output = check_source("fn f -> str\n show [[1]]\n");
        assert_eq!(codes(&output), ["T004"]);

        let output = check_source("fn f -> list i64\n sort [2 1]\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn map_show_checks_showable_bound() {
        let output = check_source("fn f xs:list i64 -> list str\n map @show xs\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn f xs:list list list i64 -> list str\n map @show xs\n");
        assert_eq!(codes(&output), ["T004"]);
    }

    #[test]
    fn fnrefs_are_valid_as_higher_order_args_and_invalid_in_let() {
        let output = check_source(
            "fn inc x:i64 -> i64\n + x 1\nfn keep x:i64 -> bool\n > x 0\nfn add a:i64 b:i64 -> i64\n + a b\nfn f xs:list i64 -> i64\n let ys map @inc xs\n let zs filter @keep ys\n fold @add 0 zs\n",
        );
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn inc x:i64 -> i64\n x\nfn f -> i64\n let g @inc\n 0\n");
        assert_eq!(codes(&output), ["T006"]);
    }

    #[test]
    fn try_requires_a_result_function_and_matching_error_type() {
        let output = check_source("fn f r:res i64 str -> res i64 str\n let x try r\n ok x\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn f r:res i64 str -> res i64 bool\n let x try r\n ok x\n");
        assert_eq!(codes(&output), ["T001"]);

        let output = check_source("fn f r:res i64 str -> i64\n try r\n");
        assert_eq!(codes(&output), ["T007"]);
    }

    #[test]
    fn else_accepts_options_and_results() {
        let output = check_source(
            "fn f o:opt i64 -> i64\n else o 0\nfn g r:res i64 str -> i64\n else r 0\n",
        );
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn main_accepts_unit_and_result_forms() {
        let output = check_source("fn main -> unit\n nil\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn main -> res unit str\n ok nil\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn main -> i64\n 0\n");
        assert_eq!(codes(&output), ["T008"]);

        let output = check_source("fn main x:i64 -> unit\n nil\n");
        assert_eq!(codes(&output), ["T008"]);
    }

    #[test]
    fn ok_without_an_expected_result_is_t010() {
        let output = check_source("fn f -> i64\n let x ok 1\n 0\n");
        assert_eq!(codes(&output), ["T010"]);
    }

    #[test]
    fn unknown_named_types_are_t002() {
        let output = check_source("fn f -> nope\n 0\n");
        assert_eq!(codes(&output), ["T002"]);
    }

    #[test]
    fn else_propagates_expected_and_accepts_result_literals() {
        let output = check_source("fn f -> i64\n else err \"x\" 0\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn f -> i64\n else ok 1 0\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn f -> i64\n else some 1 0\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn f o:opt i64 -> i64\n else o 0\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source("fn f r:res i64 str -> i64\n else r 0\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn else_infers_try_as_its_ok_side() {
        // try yields the ok side, so it is not result-shaped for else.
        let output =
            check_source("fn f r:res opt i64 str -> res i64 str\n let x else try r 0\n ok x\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output =
            check_source("fn f r:res i64 str -> res i64 str\n let x else try r 0\n ok x\n");
        assert_eq!(codes(&output), ["T001"]);
    }

    #[test]
    fn operators_unify_var_operands_before_checking() {
        let output = check_source("fn f -> res i64 str\n let x try err \"x\"\n ok + x 1\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let output = check_source(
            "fn f -> res i64 str\n let x try err \"x\"\n let y try err \"y\"\n ok + x y\n",
        );
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn fnref_to_an_effectful_builtin_is_t006() {
        let output = check_source("fn f xs:list str -> list str\n map @read xs\n");
        assert_eq!(codes(&output), ["T006"]);
    }

    #[test]
    fn ok_and_err_with_a_concrete_non_result_expected_are_t001() {
        let output = check_source("fn f -> i64\n ok 1\n");
        assert_eq!(codes(&output), ["T001"]);
        let diagnostic = &output.diagnostics[0];
        assert_eq!(diagnostic.expected.as_deref(), Some("i64"));
        assert_eq!(diagnostic.found.as_deref(), Some("res _ _"));

        let output = check_source("fn f -> str\n err \"x\"\n");
        assert_eq!(codes(&output), ["T001"]);
        let diagnostic = &output.diagnostics[0];
        assert_eq!(diagnostic.expected.as_deref(), Some("str"));
        assert_eq!(diagnostic.found.as_deref(), Some("res _ _"));
    }

    #[test]
    fn error_types_suppress_further_diagnostics() {
        let output = check_source("fn f -> nope\n [1 2]\n");
        assert_eq!(codes(&output), ["T002"]);

        let output = check_source("fn f -> i64\n let x ok 1\n x\n");
        assert_eq!(codes(&output), ["T010"]);
    }

    const SHAPE: &str = "type shape | circle r:f64 | rect w:f64 h:f64 | dot\n";

    #[test]
    fn sums_type_variants_and_match() {
        let output = check_source(&format!(
            "{SHAPE}fn f s:shape -> f64\n match s circle r r rect w h * w h dot 0.0\n"
        ));
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert_eq!(
            output.records.get("rect"),
            Some(&vec![
                ("w".to_string(), Type::F64),
                ("h".to_string(), Type::F64)
            ])
        );
        assert!(!output.records.contains_key("shape"));

        // A variant constructs the sum; the expected type flows into the arms.
        let output = check_source(&format!(
            "{SHAPE}fn f -> list shape\n [dot circle 1.0]\nfn g s:shape -> opt i64\n match s circle r none i64 rect w h some 1 dot none i64\n"
        ));
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn sum_errors() {
        // Scrutinee of another type.
        let output = check_source(&format!(
            "{SHAPE}fn f x:i64 -> i64\n match x circle r 1 rect w h 2 dot 3\n"
        ));
        assert_eq!(codes(&output), ["T001"]);
        // Arms of different types.
        let output = check_source(&format!(
            "{SHAPE}fn f s:shape -> i64\n match s circle r 1 rect w h \"x\" dot 3\n"
        ));
        assert_eq!(codes(&output), ["T001"]);
        // A variant is not a type; a sum is not comparable, showable or a record.
        let output = check_source(&format!("{SHAPE}fn f x:dot -> i64\n 1\n"));
        assert_eq!(codes(&output), ["T002"]);
        let output = check_source(&format!("{SHAPE}fn f s:shape -> bool\n == s s\n"));
        assert_eq!(codes(&output), ["T004"]);
        let output = check_source(&format!("{SHAPE}fn f s:shape -> str\n show s\n"));
        assert_eq!(codes(&output), ["T004"]);
        let output = check_source(&format!("{SHAPE}fn f s:shape -> f64\n .r s\n"));
        assert_eq!(codes(&output), ["T003"]);
    }

    #[test]
    fn recursive_sums_are_allowed() {
        let output = check_source(
            "type tree | leaf | node l:tree v:i64 r:tree\nfn f t:tree -> i64\n match t leaf 0 node l v r + v + f l f r\n",
        );
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    #[test]
    fn recursive_records_are_allowed() {
        let output =
            check_source("type node v:i64 next:opt node\nfn f -> node\n node 1 none node\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert_eq!(
            output.records.get("node"),
            Some(&vec![
                ("v".to_string(), Type::I64),
                (
                    "next".to_string(),
                    Type::Opt(Box::new(Type::Record("node".to_string())))
                ),
            ])
        );
    }

    #[test]
    fn type_display_is_canonical() {
        assert_eq!(Type::I64.to_string(), "i64");
        assert_eq!(Type::F64.to_string(), "f64");
        assert_eq!(Type::List(Box::new(Type::I64)).to_string(), "list i64");
        assert_eq!(Type::Opt(Box::new(Type::Str)).to_string(), "opt str");
        assert_eq!(
            Type::Res(Box::new(Type::Unit), Box::new(Type::Str)).to_string(),
            "res unit str"
        );
        assert_eq!(Type::Record("node".to_string()).to_string(), "node");
        assert_eq!(Type::Var(0).to_string(), "_");
        assert_eq!(Type::Error.to_string(), "error");
        assert_eq!(
            Type::Fn(vec![Type::I64, Type::Str], Box::new(Type::Bool)).to_string(),
            "(i64 str) -> bool"
        );
        assert_eq!(
            Type::Fn(Vec::new(), Box::new(Type::Unit)).to_string(),
            "() -> unit"
        );
    }

    #[test]
    fn spec_examples_type_check_without_diagnostics() {
        let programs = [
            "fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n + a b\nfn fib n:i64 -> i64\n doc \"nth fibonacci number\"\n ex fib 0 = 0\n ex fib 10 = 55\n if < n 2 n + fib - n 1 fib - n 2\nfn total xs:list i64 -> i64\n ex total [1 2 3] = 6\n ex total empty i64 = 0\n fold @add 0 xs\nfn main -> unit\n print show fib 20\n",
            "use fs.read \"data\"\nfn words s:str -> i64\n ex words \"a b c\" = 3\n len split trim s \" \"\nfn main -> res unit str\n let text try read \"data/in.txt\"\n do print show words text\n ok nil\n",
        ];
        for source in programs {
            let output = check_source(source);
            assert!(
                output.diagnostics.is_empty(),
                "SPEC §12 program had diagnostics: {:?}\n{source}",
                output.diagnostics
            );
        }
    }

    #[test]
    fn existing_valid_goldens_type_check_without_diagnostics() {
        use std::fs;
        use std::path::{Path, PathBuf};

        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("compiler/ has a parent")
            .join("tests/aph");
        let mut cases: Vec<PathBuf> = fs::read_dir(&directory)
            .expect("tests/aph exists")
            .map(|entry| entry.expect("readable dir entry").path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("aph"))
            .filter(|path| {
                path.with_extension("out").is_file() || path.with_extension("ast").is_file()
            })
            .collect();
        cases.sort();
        assert!(!cases.is_empty(), "no valid golden programs");

        for path in cases {
            let source = fs::read_to_string(&path).expect("read golden program");
            let output = check_source(&source);
            assert!(
                output.diagnostics.is_empty(),
                "{} had diagnostics: {:?}",
                path.display(),
                output.diagnostics
            );
        }
    }

    #[test]
    fn records_report_their_lowered_types() {
        let output = check_source("type p x:i64\nfn f v:p -> i64\n .x v\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert_eq!(
            output.records.get("p"),
            Some(&vec![("x".to_string(), Type::I64)])
        );
        assert_eq!(
            output.signatures.get("f"),
            Some(&(vec![Type::Record("p".to_string())], Type::I64))
        );
    }
}
