//! Aleph abstract syntax tree (SPEC §3-5, ADR-0003).
//!
//! Every node carries a stable [`NodeId`] assigned by [`assign_ids`] in
//! deterministic pre-order, and a 1-based [`Span`]. The AST is the source of
//! truth: [`File::canonicalize`] fixes the top-level order of SPEC §3.
//!
//! The `Serialize` representation is the `aleph ast` JSON: a typed AST, never
//! an untyped `serde_json::Value`. Node fields are emitted in declaration
//! order, `Span` is flattened as `line`/`col`, and each kind enum is flattened
//! as a `kind` tag plus the variant fields.

use serde::ser::Serializer;
use serde::Serialize;

use crate::lexicon;

/// A stable node identifier, serialized as `n0`, `n1`, ...
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// The placeholder used before [`assign_ids`] runs.
    pub const UNASSIGNED: NodeId = NodeId(u32::MAX);
}

impl Serialize for NodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("n{}", self.0))
    }
}

/// A 1-based source position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

/// A capability introduced by `use` (SPEC §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    FsRead,
    FsWrite,
    NetHost,
    Env,
    Time,
}

impl Capability {
    /// Index into [`lexicon::CAPABILITIES`].
    const fn index(self) -> usize {
        match self {
            Self::FsRead => 0,
            Self::FsWrite => 1,
            Self::NetHost => 2,
            Self::Env => 3,
            Self::Time => 4,
        }
    }

    /// Canonical spelling from the lexicon.
    pub fn spelling(self) -> &'static str {
        lexicon::CAPABILITIES[self.index()]
    }

    /// Parse a capability name, e.g. `"fs.read"`.
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        [
            Self::FsRead,
            Self::FsWrite,
            Self::NetHost,
            Self::Env,
            Self::Time,
        ]
        .into_iter()
        .find(|cap| cap.spelling() == spelling)
    }
}

impl Serialize for Capability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.spelling())
    }
}

/// Binary operators (SPEC §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    /// Canonical spelling from the lexicon.
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Add => lexicon::operator("add"),
            Self::Sub => lexicon::operator("sub"),
            Self::Mul => lexicon::operator("mul"),
            Self::Div => lexicon::operator("div"),
            Self::Mod => lexicon::operator("mod"),
            Self::Eq => lexicon::operator("eq"),
            Self::Ne => lexicon::operator("ne"),
            Self::Lt => lexicon::operator("lt"),
            Self::Le => lexicon::operator("le"),
            Self::Gt => lexicon::operator("gt"),
            Self::Ge => lexicon::operator("ge"),
            Self::And => lexicon::keyword("land"),
            Self::Or => lexicon::keyword("lor"),
        }
        .expect("lexicon entry for operator")
    }
}

impl Serialize for BinOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.spelling())
    }
}

/// Unary operators (SPEC §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
}

impl UnOp {
    /// Canonical spelling from the lexicon.
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Not => lexicon::keyword("lnot"),
            Self::Neg => lexicon::keyword("negate"),
        }
        .expect("lexicon entry for operator")
    }
}

impl Serialize for UnOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.spelling())
    }
}

/// The file root.
#[derive(Debug, Clone, Serialize)]
pub struct File {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub uses: Vec<Use>,
    pub types: Vec<TypeDef>,
    pub fns: Vec<Fn>,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.uses == other.uses && self.types == other.types && self.fns == other.fns
    }
}

/// A `use` item.
#[derive(Debug, Clone, Serialize)]
pub struct Use {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub cap: Capability,
    pub path: Option<String>,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Use {
    fn eq(&self, other: &Self) -> bool {
        self.cap == other.cap && self.path == other.path
    }
}

/// A `type` item.
#[derive(Debug, Clone, Serialize)]
pub struct TypeDef {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub name: String,
    /// Record fields; empty for a sum type.
    pub fields: Vec<Field>,
    /// Sum variants (ADR-0034); empty for a record.
    pub variants: Vec<Variant>,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for TypeDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.fields == other.fields && self.variants == other.variants
    }
}

/// One variant of a sum type: a constructor with its fields (ADR-0034).
#[derive(Debug, Clone, Serialize)]
pub struct Variant {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub name: String,
    pub fields: Vec<Field>,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.fields == other.fields
    }
}

/// One record field.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub name: String,
    pub ty: Type,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.ty == other.ty
    }
}

/// A `fn` item.
#[derive(Debug, Clone, Serialize)]
pub struct Fn {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    /// Always present in JSON (`null` when the function has no `doc`).
    pub doc: Option<String>,
    pub examples: Vec<Example>,
    pub stmts: Vec<Stmt>,
    pub result: Expr,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Fn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.ret == other.ret
            && self.doc == other.doc
            && self.examples == other.examples
            && self.stmts == other.stmts
            && self.result == other.result
    }
}

/// One function parameter.
#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub name: String,
    pub ty: Type,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Param {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.ty == other.ty
    }
}

/// One `ex` example.
#[derive(Debug, Clone, Serialize)]
pub struct Example {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    pub lhs: Expr,
    pub rhs: Expr,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Example {
    fn eq(&self, other: &Self) -> bool {
        self.lhs == other.lhs && self.rhs == other.rhs
    }
}

/// One body statement.
#[derive(Debug, Clone, Serialize)]
pub struct Stmt {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    #[serde(flatten)]
    pub kind: StmtKind,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

/// Statement forms (SPEC §3.3).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StmtKind {
    Let {
        name: String,
        value: Expr,
    },
    Do {
        value: Expr,
    },
    /// `if C E`: return `E` when `C` is true (ADR-0044).
    Guard {
        cond: Expr,
        value: Expr,
    },
}

/// A type expression.
#[derive(Debug, Clone, Serialize)]
pub struct Type {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    #[serde(flatten)]
    pub kind: TypeKind,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

/// Type forms (SPEC §4).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeKind {
    I64,
    F64,
    Bool,
    Str,
    Unit,
    /// `json` (ADR-0033).
    Json,
    List {
        elem: Box<Type>,
    },
    Opt {
        elem: Box<Type>,
    },
    Res {
        ok: Box<Type>,
        err: Box<Type>,
    },
    /// `dict K V` (ADR-0032).
    Dict {
        key: Box<Type>,
        val: Box<Type>,
    },
    Named {
        name: String,
    },
}

/// An expression.
#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub id: NodeId,
    #[serde(flatten)]
    pub span: Span,
    #[serde(flatten)]
    pub kind: ExprKind,
}

// when adding a field to a node, add it here (round-trip tests depend on it)
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

/// What a call target is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CalleeKind {
    Fn,
    Builtin,
    Ctor,
}

/// Expression forms (SPEC §5).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExprKind {
    Int {
        value: i64,
    },
    /// Canonical token text, so `1.0` and `1.00` remain distinguishable.
    Float {
        value: String,
    },
    Str {
        value: String,
    },
    Bool {
        value: bool,
    },
    Nil,
    Var {
        name: String,
    },
    FnRef {
        name: String,
    },
    Call {
        callee: String,
        callee_kind: CalleeKind,
        args: Vec<Expr>,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnOp {
        op: UnOp,
        operand: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        #[serde(rename = "then")]
        then_: Box<Expr>,
        #[serde(rename = "else")]
        else_: Box<Expr>,
    },
    List {
        elems: Vec<Expr>,
    },
    Empty {
        ty: Type,
    },
    /// `[]`: an empty list whose element type comes from the context (ADR-0041).
    EmptyList,
    /// The empty dict `dict K V`; `ty` is the dict type (ADR-0032).
    Dict {
        ty: Type,
    },
    Field {
        name: String,
        value: Box<Expr>,
    },
    Some {
        value: Box<Expr>,
    },
    None {
        ty: Type,
    },
    Ok {
        value: Box<Expr>,
    },
    Err {
        value: Box<Expr>,
    },
    Try {
        value: Box<Expr>,
    },
    Else {
        value: Box<Expr>,
        default: Box<Expr>,
    },
    Has {
        value: Box<Expr>,
    },
    IsOk {
        value: Box<Expr>,
    },
    /// `match e ARM..`: one arm per variant, in declaration order (ADR-0034).
    Match {
        value: Box<Expr>,
        arms: Vec<Arm>,
    },
    /// `lam x.. BODY`: an inline function, only as the function argument of
    /// a higher-order builtin (ADR-0037).
    Lam {
        binders: Vec<String>,
        body: Box<Expr>,
    },
}

/// One `match` arm: the variant, one binder per field, and the body.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Arm {
    pub variant: String,
    pub binders: Vec<String>,
    pub body: Expr,
}

impl File {
    /// Sort top-level items into canonical order (SPEC §3).
    ///
    /// `use` items are sorted by their full canonical line text, `type` items
    /// by name, and `fn` items by name with `main` last. Sorting is stable, so
    /// equal names keep their source order.
    ///
    /// Call this *before* [`assign_ids`], or use [`File::finish`], which does
    /// both in that order.
    pub fn canonicalize(&mut self) {
        self.uses.sort_by_cached_key(Use::canonical_text);
        self.types.sort_by(|a, b| a.name.cmp(&b.name));
        self.fns.sort_by(|a, b| {
            let a_key = (a.name == "main", a.name.as_str());
            let b_key = (b.name == "main", b.name.as_str());
            a_key.cmp(&b_key)
        });
    }

    /// Put the file into its final form: canonicalize, then assign ids.
    pub fn finish(&mut self) {
        self.canonicalize();
        assign_ids(self);
    }

    /// Whether both files have the same shape, values and node ids. Spans are
    /// ignored. Intended for tests.
    pub fn same_ids(&self, other: &File) -> bool {
        self == other && collect_ids(self) == collect_ids(other)
    }
}

/// Escape a string for canonical printing: `\n`, `\t`, `"` and `\`.
pub fn escape_str(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

impl Use {
    /// Canonical text of this `use` line (SPEC §3.1).
    ///
    /// This is the single implementation shared by the printer and by the
    /// SPEC §3 sort key in [`File::canonicalize`].
    pub fn canonical_text(&self) -> String {
        let mut text = String::new();
        text.push_str(lexicon::keyword("capability").expect("lexicon entry for use"));
        text.push(' ');
        text.push_str(self.cap.spelling());
        if let Some(path) = &self.path {
            text.push(' ');
            text.push('"');
            text.push_str(&escape_str(path));
            text.push('"');
        }
        text
    }
}

/// Assign node ids in deterministic pre-order from `n0` (ADR-0003).
///
/// Order: `File`, uses, type definitions and their fields/variants/types, then
/// functions: `Fn`, params/types, return type, examples, statements, result.
///
/// Call this *after* [`File::canonicalize`], or use [`File::finish`], which
/// does both in that order.
pub fn assign_ids(file: &mut File) {
    let mut next = 0;
    assign_file(file, &mut next);
}

fn fresh(next: &mut u32) -> NodeId {
    let id = *next;
    *next += 1;
    NodeId(id)
}

fn assign_file(file: &mut File, next: &mut u32) {
    file.id = fresh(next);
    for item in &mut file.uses {
        item.id = fresh(next);
    }
    for item in &mut file.types {
        assign_typedef(item, next);
    }
    for item in &mut file.fns {
        assign_fn(item, next);
    }
}

fn assign_typedef(item: &mut TypeDef, next: &mut u32) {
    item.id = fresh(next);
    assign_fields(&mut item.fields, next);
    for variant in &mut item.variants {
        variant.id = fresh(next);
        assign_fields(&mut variant.fields, next);
    }
}

fn assign_fields(fields: &mut [Field], next: &mut u32) {
    for field in fields {
        field.id = fresh(next);
        assign_type(&mut field.ty, next);
    }
}

fn assign_fn(item: &mut Fn, next: &mut u32) {
    item.id = fresh(next);
    for param in &mut item.params {
        param.id = fresh(next);
        assign_type(&mut param.ty, next);
    }
    assign_type(&mut item.ret, next);
    for example in &mut item.examples {
        example.id = fresh(next);
        assign_expr(&mut example.lhs, next);
        assign_expr(&mut example.rhs, next);
    }
    for stmt in &mut item.stmts {
        stmt.id = fresh(next);
        match &mut stmt.kind {
            StmtKind::Let { value, .. } => assign_expr(value, next),
            StmtKind::Do { value } => assign_expr(value, next),
            StmtKind::Guard { cond, value } => {
                assign_expr(cond, next);
                assign_expr(value, next);
            }
        }
    }
    assign_expr(&mut item.result, next);
}

fn assign_type(ty: &mut Type, next: &mut u32) {
    ty.id = fresh(next);
    match &mut ty.kind {
        TypeKind::List { elem } | TypeKind::Opt { elem } => assign_type(elem, next),
        TypeKind::Res { ok, err } => {
            assign_type(ok, next);
            assign_type(err, next);
        }
        TypeKind::Dict { key, val } => {
            assign_type(key, next);
            assign_type(val, next);
        }
        TypeKind::I64
        | TypeKind::F64
        | TypeKind::Bool
        | TypeKind::Str
        | TypeKind::Unit
        | TypeKind::Json
        | TypeKind::Named { .. } => {}
    }
}

fn assign_expr(expr: &mut Expr, next: &mut u32) {
    expr.id = fresh(next);
    match &mut expr.kind {
        ExprKind::Call { args, .. } => {
            for arg in args {
                assign_expr(arg, next);
            }
        }
        ExprKind::BinOp { lhs, rhs, .. } => {
            assign_expr(lhs, next);
            assign_expr(rhs, next);
        }
        ExprKind::UnOp { operand, .. } => assign_expr(operand, next),
        ExprKind::If { cond, then_, else_ } => {
            assign_expr(cond, next);
            assign_expr(then_, next);
            assign_expr(else_, next);
        }
        ExprKind::List { elems } => {
            for elem in elems {
                assign_expr(elem, next);
            }
        }
        ExprKind::Empty { ty } | ExprKind::None { ty } | ExprKind::Dict { ty } => {
            assign_type(ty, next)
        }
        ExprKind::Field { value, .. }
        | ExprKind::Some { value }
        | ExprKind::Ok { value }
        | ExprKind::Err { value }
        | ExprKind::Try { value }
        | ExprKind::Has { value }
        | ExprKind::IsOk { value } => assign_expr(value, next),
        ExprKind::Else { value, default } => {
            assign_expr(value, next);
            assign_expr(default, next);
        }
        ExprKind::Match { value, arms } => {
            assign_expr(value, next);
            for arm in arms {
                assign_expr(&mut arm.body, next);
            }
        }
        ExprKind::Lam { body, .. } => assign_expr(body, next),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Str { .. }
        | ExprKind::Bool { .. }
        | ExprKind::Nil
        | ExprKind::EmptyList
        | ExprKind::Var { .. }
        | ExprKind::FnRef { .. } => {}
    }
}

pub(crate) fn collect_ids(file: &File) -> Vec<NodeId> {
    let mut ids = Vec::new();
    collect_file(file, &mut ids);
    ids
}

fn collect_file(file: &File, ids: &mut Vec<NodeId>) {
    ids.push(file.id);
    for item in &file.uses {
        ids.push(item.id);
    }
    for item in &file.types {
        collect_typedef(item, ids);
    }
    for item in &file.fns {
        collect_fn(item, ids);
    }
}

fn collect_typedef(item: &TypeDef, ids: &mut Vec<NodeId>) {
    ids.push(item.id);
    collect_fields(&item.fields, ids);
    for variant in &item.variants {
        ids.push(variant.id);
        collect_fields(&variant.fields, ids);
    }
}

fn collect_fields(fields: &[Field], ids: &mut Vec<NodeId>) {
    for field in fields {
        ids.push(field.id);
        collect_type(&field.ty, ids);
    }
}

fn collect_fn(item: &Fn, ids: &mut Vec<NodeId>) {
    ids.push(item.id);
    for param in &item.params {
        ids.push(param.id);
        collect_type(&param.ty, ids);
    }
    collect_type(&item.ret, ids);
    for example in &item.examples {
        ids.push(example.id);
        collect_expr(&example.lhs, ids);
        collect_expr(&example.rhs, ids);
    }
    for stmt in &item.stmts {
        ids.push(stmt.id);
        match &stmt.kind {
            StmtKind::Let { value, .. } => collect_expr(value, ids),
            StmtKind::Do { value } => collect_expr(value, ids),
            StmtKind::Guard { cond, value } => {
                collect_expr(cond, ids);
                collect_expr(value, ids);
            }
        }
    }
    collect_expr(&item.result, ids);
}

fn collect_type(ty: &Type, ids: &mut Vec<NodeId>) {
    ids.push(ty.id);
    match &ty.kind {
        TypeKind::List { elem } | TypeKind::Opt { elem } => collect_type(elem, ids),
        TypeKind::Res { ok, err } => {
            collect_type(ok, ids);
            collect_type(err, ids);
        }
        TypeKind::Dict { key, val } => {
            collect_type(key, ids);
            collect_type(val, ids);
        }
        TypeKind::I64
        | TypeKind::F64
        | TypeKind::Bool
        | TypeKind::Str
        | TypeKind::Unit
        | TypeKind::Json
        | TypeKind::Named { .. } => {}
    }
}

fn collect_expr(expr: &Expr, ids: &mut Vec<NodeId>) {
    ids.push(expr.id);
    match &expr.kind {
        ExprKind::Call { args, .. } => {
            for arg in args {
                collect_expr(arg, ids);
            }
        }
        ExprKind::BinOp { lhs, rhs, .. } => {
            collect_expr(lhs, ids);
            collect_expr(rhs, ids);
        }
        ExprKind::UnOp { operand, .. } => collect_expr(operand, ids),
        ExprKind::If { cond, then_, else_ } => {
            collect_expr(cond, ids);
            collect_expr(then_, ids);
            collect_expr(else_, ids);
        }
        ExprKind::List { elems } => {
            for elem in elems {
                collect_expr(elem, ids);
            }
        }
        ExprKind::Empty { ty } | ExprKind::None { ty } | ExprKind::Dict { ty } => {
            collect_type(ty, ids)
        }
        ExprKind::Field { value, .. }
        | ExprKind::Some { value }
        | ExprKind::Ok { value }
        | ExprKind::Err { value }
        | ExprKind::Try { value }
        | ExprKind::Has { value }
        | ExprKind::IsOk { value } => collect_expr(value, ids),
        ExprKind::Else { value, default } => {
            collect_expr(value, ids);
            collect_expr(default, ids);
        }
        ExprKind::Match { value, arms } => {
            collect_expr(value, ids);
            for arm in arms {
                collect_expr(&arm.body, ids);
            }
        }
        ExprKind::Lam { body, .. } => collect_expr(body, ids),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Str { .. }
        | ExprKind::Bool { .. }
        | ExprKind::Nil
        | ExprKind::EmptyList
        | ExprKind::Var { .. }
        | ExprKind::FnRef { .. } => {}
    }
}

/// Walk `expr` in pre-order, calling `visit` on every expression node.
///
/// Children are visited left to right: call arguments, binary operands, unary
/// operand, `if` condition then branches, list elements, wrapped values,
/// `else` value then default, `match` scrutinee then arm bodies, and `lam` body. Types inside `empty`/`none` are not expressions.
pub fn walk_expr<'a>(expr: &'a Expr, visit: &mut impl FnMut(&'a Expr)) {
    visit(expr);
    match &expr.kind {
        ExprKind::Call { args, .. } => {
            for arg in args {
                walk_expr(arg, visit);
            }
        }
        ExprKind::BinOp { lhs, rhs, .. } => {
            walk_expr(lhs, visit);
            walk_expr(rhs, visit);
        }
        ExprKind::UnOp { operand, .. } => walk_expr(operand, visit),
        ExprKind::If { cond, then_, else_ } => {
            walk_expr(cond, visit);
            walk_expr(then_, visit);
            walk_expr(else_, visit);
        }
        ExprKind::List { elems } => {
            for elem in elems {
                walk_expr(elem, visit);
            }
        }
        ExprKind::Field { value, .. }
        | ExprKind::Some { value }
        | ExprKind::Ok { value }
        | ExprKind::Err { value }
        | ExprKind::Try { value }
        | ExprKind::Has { value }
        | ExprKind::IsOk { value } => walk_expr(value, visit),
        ExprKind::Else { value, default } => {
            walk_expr(value, visit);
            walk_expr(default, visit);
        }
        ExprKind::Match { value, arms } => {
            walk_expr(value, visit);
            for arm in arms {
                walk_expr(&arm.body, visit);
            }
        }
        ExprKind::Lam { body, .. } => walk_expr(body, visit),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Str { .. }
        | ExprKind::Bool { .. }
        | ExprKind::Nil
        | ExprKind::EmptyList
        | ExprKind::Var { .. }
        | ExprKind::FnRef { .. }
        | ExprKind::Empty { .. }
        | ExprKind::Dict { .. }
        | ExprKind::None { .. } => {}
    }
}

/// Like [`walk_expr`], but with mutable access. `visit` runs before the
/// children are walked, so a node whose kind it replaces is walked in its new
/// form.
pub fn walk_expr_mut(expr: &mut Expr, visit: &mut impl FnMut(&mut Expr)) {
    visit(expr);
    match &mut expr.kind {
        ExprKind::Call { args, .. } => {
            for arg in args {
                walk_expr_mut(arg, visit);
            }
        }
        ExprKind::BinOp { lhs, rhs, .. } => {
            walk_expr_mut(lhs, visit);
            walk_expr_mut(rhs, visit);
        }
        ExprKind::UnOp { operand, .. } => walk_expr_mut(operand, visit),
        ExprKind::If { cond, then_, else_ } => {
            walk_expr_mut(cond, visit);
            walk_expr_mut(then_, visit);
            walk_expr_mut(else_, visit);
        }
        ExprKind::List { elems } => {
            for elem in elems {
                walk_expr_mut(elem, visit);
            }
        }
        ExprKind::Field { value, .. }
        | ExprKind::Some { value }
        | ExprKind::Ok { value }
        | ExprKind::Err { value }
        | ExprKind::Try { value }
        | ExprKind::Has { value }
        | ExprKind::IsOk { value } => walk_expr_mut(value, visit),
        ExprKind::Else { value, default } => {
            walk_expr_mut(value, visit);
            walk_expr_mut(default, visit);
        }
        ExprKind::Match { value, arms } => {
            walk_expr_mut(value, visit);
            for arm in arms {
                walk_expr_mut(&mut arm.body, visit);
            }
        }
        ExprKind::Lam { body, .. } => walk_expr_mut(body, visit),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Str { .. }
        | ExprKind::Bool { .. }
        | ExprKind::Nil
        | ExprKind::EmptyList
        | ExprKind::Var { .. }
        | ExprKind::FnRef { .. }
        | ExprKind::Empty { .. }
        | ExprKind::Dict { .. }
        | ExprKind::None { .. } => {}
    }
}

/// Serialize a file as pretty JSON with a trailing newline (`aleph ast`).
pub fn to_json(file: &File) -> String {
    debug_assert!(
        collect_ids(file).iter().all(|id| *id != NodeId::UNASSIGNED),
        "call File::finish (canonicalize then assign_ids) before serializing"
    );
    let mut text = serde_json::to_string_pretty(file).expect("AST serializes to JSON");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span { line: 1, col: 1 }
    }

    fn ty(kind: TypeKind) -> Type {
        Type {
            id: NodeId::UNASSIGNED,
            span: span(),
            kind,
        }
    }

    fn expr(kind: ExprKind) -> Expr {
        Expr {
            id: NodeId::UNASSIGNED,
            span: span(),
            kind,
        }
    }

    fn var(name: &str) -> Expr {
        expr(ExprKind::Var {
            name: name.to_string(),
        })
    }

    fn int(value: i64) -> Expr {
        expr(ExprKind::Int { value })
    }

    fn add_file() -> File {
        File {
            id: NodeId::UNASSIGNED,
            span: span(),
            uses: Vec::new(),
            types: Vec::new(),
            fns: vec![Fn {
                id: NodeId::UNASSIGNED,
                span: span(),
                name: "add".to_string(),
                params: vec![
                    Param {
                        id: NodeId::UNASSIGNED,
                        span: span(),
                        name: "a".to_string(),
                        ty: ty(TypeKind::I64),
                    },
                    Param {
                        id: NodeId::UNASSIGNED,
                        span: span(),
                        name: "b".to_string(),
                        ty: ty(TypeKind::I64),
                    },
                ],
                ret: ty(TypeKind::I64),
                doc: None,
                examples: vec![Example {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    lhs: expr(ExprKind::Call {
                        callee: "add".to_string(),
                        callee_kind: CalleeKind::Fn,
                        args: vec![int(2), int(3)],
                    }),
                    rhs: int(5),
                }],
                stmts: Vec::new(),
                result: expr(ExprKind::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(var("a")),
                    rhs: Box::new(var("b")),
                }),
            }],
        }
    }

    #[test]
    fn assign_ids_marks_the_add_example_in_preorder() {
        let mut file = add_file();
        assign_ids(&mut file);

        let function = &file.fns[0];
        assert_eq!(file.id, NodeId(0));
        assert_eq!(function.id, NodeId(1));
        assert_eq!(function.params[0].id, NodeId(2));
        assert_eq!(function.params[0].ty.id, NodeId(3));
        assert_eq!(function.params[1].id, NodeId(4));
        assert_eq!(function.params[1].ty.id, NodeId(5));
        assert_eq!(function.ret.id, NodeId(6));
        assert_eq!(function.examples[0].id, NodeId(7));
        let lhs = &function.examples[0].lhs;
        assert_eq!(lhs.id, NodeId(8));
        let ExprKind::Call { args, .. } = &lhs.kind else {
            panic!("expected call");
        };
        assert_eq!(args[0].id, NodeId(9));
        assert_eq!(args[1].id, NodeId(10));
        assert_eq!(function.examples[0].rhs.id, NodeId(11));
        let result = &function.result;
        assert_eq!(result.id, NodeId(12));
        let ExprKind::BinOp { lhs, rhs, .. } = &result.kind else {
            panic!("expected binop");
        };
        assert_eq!(lhs.id, NodeId(13));
        assert_eq!(rhs.id, NodeId(14));

        let expected: Vec<NodeId> = (0..=14).map(NodeId).collect();
        assert_eq!(collect_ids(&file), expected);
    }

    #[test]
    fn add_example_json_is_exact() {
        let mut file = add_file();
        assign_ids(&mut file);
        let expected = r#"{
  "id": "n0",
  "line": 1,
  "col": 1,
  "uses": [],
  "types": [],
  "fns": [
    {
      "id": "n1",
      "line": 1,
      "col": 1,
      "name": "add",
      "params": [
        {
          "id": "n2",
          "line": 1,
          "col": 1,
          "name": "a",
          "ty": {
            "id": "n3",
            "line": 1,
            "col": 1,
            "kind": "i64"
          }
        },
        {
          "id": "n4",
          "line": 1,
          "col": 1,
          "name": "b",
          "ty": {
            "id": "n5",
            "line": 1,
            "col": 1,
            "kind": "i64"
          }
        }
      ],
      "ret": {
        "id": "n6",
        "line": 1,
        "col": 1,
        "kind": "i64"
      },
      "doc": null,
      "examples": [
        {
          "id": "n7",
          "line": 1,
          "col": 1,
          "lhs": {
            "id": "n8",
            "line": 1,
            "col": 1,
            "kind": "call",
            "callee": "add",
            "callee_kind": "fn",
            "args": [
              {
                "id": "n9",
                "line": 1,
                "col": 1,
                "kind": "int",
                "value": 2
              },
              {
                "id": "n10",
                "line": 1,
                "col": 1,
                "kind": "int",
                "value": 3
              }
            ]
          },
          "rhs": {
            "id": "n11",
            "line": 1,
            "col": 1,
            "kind": "int",
            "value": 5
          }
        }
      ],
      "stmts": [],
      "result": {
        "id": "n12",
        "line": 1,
        "col": 1,
        "kind": "bin_op",
        "op": "+",
        "lhs": {
          "id": "n13",
          "line": 1,
          "col": 1,
          "kind": "var",
          "name": "a"
        },
        "rhs": {
          "id": "n14",
          "line": 1,
          "col": 1,
          "kind": "var",
          "name": "b"
        }
      }
    }
  ]
}
"#;
        assert_eq!(to_json(&file), expected);
    }

    #[test]
    fn canonicalize_sorts_items_and_puts_main_last() {
        let mut file = File {
            id: NodeId::UNASSIGNED,
            span: span(),
            uses: vec![
                Use {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    cap: Capability::FsWrite,
                    path: Some("b".to_string()),
                },
                Use {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    cap: Capability::FsRead,
                    path: Some("a".to_string()),
                },
                Use {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    cap: Capability::Env,
                    path: None,
                },
            ],
            types: vec![
                TypeDef {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    name: "z".to_string(),
                    fields: Vec::new(),
                    variants: Vec::new(),
                },
                TypeDef {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    name: "a".to_string(),
                    fields: Vec::new(),
                    variants: Vec::new(),
                },
            ],
            fns: ["main", "zeta", "alpha"]
                .into_iter()
                .map(|name| Fn {
                    id: NodeId::UNASSIGNED,
                    span: span(),
                    name: name.to_string(),
                    params: Vec::new(),
                    ret: ty(TypeKind::Unit),
                    doc: None,
                    examples: Vec::new(),
                    stmts: Vec::new(),
                    result: expr(ExprKind::Nil),
                })
                .collect(),
        };
        file.canonicalize();

        let uses: Vec<_> = file.uses.iter().map(Use::canonical_text).collect();
        assert_eq!(
            uses,
            vec![
                "use env".to_string(),
                "use fs.read \"a\"".to_string(),
                "use fs.write \"b\"".to_string(),
            ]
        );
        let types: Vec<_> = file.types.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(types, vec!["a", "z"]);
        let fns: Vec<_> = file.fns.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(fns, vec!["alpha", "zeta", "main"]);
    }

    #[test]
    fn use_canonical_text_escapes_quotes_and_newlines() {
        let item = Use {
            id: NodeId::UNASSIGNED,
            span: span(),
            cap: Capability::FsRead,
            path: Some("a\"b\nc\td\\e".to_string()),
        };
        assert_eq!(item.canonical_text(), "use fs.read \"a\\\"b\\nc\\td\\\\e\"");
    }

    #[test]
    fn finish_canonicalizes_then_assigns_ids_in_collect_order() {
        let mut file = add_file();
        file.finish();

        let ids = collect_ids(&file);
        let expected: Vec<NodeId> = (0..ids.len() as u32).map(NodeId).collect();
        assert_eq!(ids, expected);
        assert!(ids.iter().all(|id| *id != NodeId::UNASSIGNED));
        assert!(to_json(&file).contains("\"id\": \"n0\""));
    }

    #[test]
    fn equality_ignores_ids_and_spans() {
        let mut first = add_file();
        assign_ids(&mut first);

        let mut second = add_file();
        second.span = Span { line: 9, col: 9 };
        second.fns[0].span = Span { line: 8, col: 8 };
        second.fns[0].result.span = Span { line: 7, col: 7 };

        assert_eq!(first, second);
        assert!(!first.same_ids(&second));
        second = first.clone();
        assert!(first.same_ids(&second));
    }

    #[test]
    fn walk_expr_visits_nodes_in_preorder() {
        let expr = expr(ExprKind::BinOp {
            op: BinOp::Add,
            lhs: Box::new(int(1)),
            rhs: Box::new(expr(ExprKind::Call {
                callee: "f".to_string(),
                callee_kind: CalleeKind::Fn,
                args: vec![var("a")],
            })),
        });

        let mut tags = Vec::new();
        walk_expr(&expr, &mut |node| {
            tags.push(match &node.kind {
                ExprKind::BinOp { .. } => "binop",
                ExprKind::Int { .. } => "int",
                ExprKind::Call { .. } => "call",
                ExprKind::Var { .. } => "var",
                _ => "other",
            });
        });
        assert_eq!(tags, ["binop", "int", "call", "var"]);
    }

    #[test]
    fn do_statement_serializes_its_value_as_a_nested_node() {
        let mut file = add_file();
        file.fns[0].stmts.push(Stmt {
            id: NodeId::UNASSIGNED,
            span: span(),
            kind: StmtKind::Do {
                value: expr(ExprKind::Nil),
            },
        });
        assign_ids(&mut file);

        let json: serde_json::Value = serde_json::from_str(&to_json(&file)).unwrap();
        let stmt = &json["fns"][0]["stmts"][0];
        assert_eq!(stmt["kind"], "do");
        assert_eq!(stmt["value"]["kind"], "nil");
        assert_eq!(stmt["value"]["id"], "n13");
        assert_eq!(json["fns"][0]["result"]["id"], "n14");
    }
}
