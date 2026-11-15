//! Aleph parser (SPEC §3-5).
//!
//! Parsing is arity-directed (ADR-0004). Pass 1 collects top-level `use`,
//! `type` and `fn` headers, plus the function signatures and record
//! constructor arities that pass 2 needs. Pass 2 then parses each function body
//! (`doc`, `ex`, `let`, `do` and the result line) into expressions.
//!
//! Types are parsed in pass 1 and reused by `empty`/`none` in pass 2. A
//! function gets a placeholder [`ast::ExprKind::Nil`] result during pass 1;
//! pass 2 replaces it with the parsed result.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{self, NodeId};
use crate::builtins;
use crate::diag::Diagnostic;
use crate::lexer::{self, Line, LineKind, Token, TokenKind};
use crate::lexicon;

/// Internal parser result: a diagnostic is boxed because `Diagnostic` is large.
type PResult<T> = Result<T, Box<Diagnostic>>;

/// The result of parsing a source file.
#[derive(Debug)]
pub struct ParseOutput {
    /// The AST, or `None` when there are diagnostics (lexical or parse).
    pub file: Option<ast::File>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Function signatures and record constructor arities collected in pass 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Signatures {
    functions: BTreeMap<String, usize>,
    records: BTreeMap<String, usize>,
}

/// Lex `source`, then parse it. Lexical errors stop parsing: the token stream
/// is unreliable after them.
pub fn parse(file: &str, source: &str) -> ParseOutput {
    let (lines, diagnostics) = lexer::lex(file, source);
    if !diagnostics.is_empty() {
        return ParseOutput {
            file: None,
            diagnostics,
        };
    }
    parse_lines(file, &lines)
}

/// Parse lexer lines (used by tests, and by `parse` after lexing).
///
/// Runs pass 1, then pass 2, then [`ast::File::finish`].
pub fn parse_lines(file: &str, lines: &[Line]) -> ParseOutput {
    let mut parser = Parser::new(file, lines);
    parser.collect_headers();
    parser.parse_bodies();

    if parser.diagnostics.is_empty() {
        let mut ast = parser.ast;
        ast.finish();
        ParseOutput {
            file: Some(ast),
            diagnostics: Vec::new(),
        }
    } else {
        let mut diagnostics = parser.diagnostics;
        diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.col));
        ParseOutput {
            file: None,
            diagnostics,
        }
    }
}

#[derive(Debug, Clone)]
enum BodyState {
    /// Body lines belong to this function.
    Attach(String),
    /// The preceding item was malformed: silently skip its body lines.
    Skip,
    /// No function is open: a body line here is an error.
    Error,
}

struct Parser<'a> {
    file: &'a str,
    lines: &'a [Line],
    ast: ast::File,
    signatures: Signatures,
    /// Raw body lines per function name, consumed by pass 2 (T-007).
    bodies: BTreeMap<String, Vec<Line>>,
    diagnostics: Vec<Diagnostic>,
    /// Where indented body lines go.
    body_state: BodyState,
}

impl<'a> Parser<'a> {
    fn new(file: &'a str, lines: &'a [Line]) -> Self {
        Self {
            file,
            lines,
            ast: ast::File {
                id: NodeId::UNASSIGNED,
                span: ast::Span { line: 1, col: 1 },
                uses: Vec::new(),
                types: Vec::new(),
                fns: Vec::new(),
            },
            signatures: Signatures::default(),
            bodies: BTreeMap::new(),
            diagnostics: Vec::new(),
            body_state: BodyState::Error,
        }
    }

    fn collect_headers(&mut self) {
        for index in 0..self.lines.len() {
            let line = &self.lines[index];

            if line.kind == LineKind::Body {
                match &self.body_state {
                    BodyState::Attach(name) => {
                        let name = name.clone();
                        self.bodies.entry(name).or_default().push(line.clone());
                    }
                    BodyState::Skip => {}
                    BodyState::Error => {
                        if let Some(first) = line.tokens.first() {
                            self.diagnostics.push(*p001(
                                self.file,
                                first,
                                "body line outside function",
                            ));
                        }
                    }
                }
                continue;
            }

            self.body_state = BodyState::Error;
            let Some(first) = line.tokens.first() else {
                continue;
            };

            if is_keyword(first, "capability") {
                match parse_use(self.file, line) {
                    Ok(item) => self.ast.uses.push(item),
                    Err(diagnostic) => {
                        self.diagnostics.push(*diagnostic);
                        self.body_state = BodyState::Skip;
                    }
                }
            } else if is_keyword(first, "record") {
                match parse_typedef(self.file, line) {
                    Ok(item) if is_reserved_name(&item.name) => {
                        let name_token = &line.tokens[1];
                        self.diagnostics
                            .push(*p001(self.file, name_token, "reserved name"));
                        self.body_state = BodyState::Skip;
                    }
                    Ok(item)
                        if self.signatures.records.contains_key(&item.name)
                            || self.signatures.functions.contains_key(&item.name) =>
                    {
                        let name_token = &line.tokens[1];
                        self.diagnostics.push(*duplicate(self.file, name_token));
                        self.body_state = BodyState::Skip;
                    }
                    Ok(item) => {
                        self.signatures
                            .records
                            .insert(item.name.clone(), item.fields.len());
                        self.ast.types.push(item);
                    }
                    Err(diagnostic) => {
                        self.diagnostics.push(*diagnostic);
                        self.body_state = BodyState::Skip;
                    }
                }
            } else if is_keyword(first, "function") {
                match parse_fn(self.file, line) {
                    Ok(item) if is_reserved_name(&item.name) => {
                        let name_token = &line.tokens[1];
                        self.diagnostics
                            .push(*p001(self.file, name_token, "reserved name"));
                        self.body_state = BodyState::Skip;
                    }
                    Ok(item)
                        if self.signatures.functions.contains_key(&item.name)
                            || self.signatures.records.contains_key(&item.name) =>
                    {
                        let name_token = &line.tokens[1];
                        self.diagnostics.push(*duplicate(self.file, name_token));
                        self.body_state = BodyState::Skip;
                    }
                    Ok(item) => {
                        self.signatures
                            .functions
                            .insert(item.name.clone(), item.params.len());
                        let name = item.name.clone();
                        self.bodies.entry(name.clone()).or_default();
                        self.ast.fns.push(item);
                        self.body_state = BodyState::Attach(name);
                    }
                    Err(diagnostic) => {
                        self.diagnostics.push(*diagnostic);
                        self.body_state = BodyState::Skip;
                    }
                }
            } else {
                self.diagnostics
                    .push(*p001(self.file, first, "unexpected token"));
            }
        }
    }
}

/// Maximum nesting depth of an expression, to keep the recursive parser from
/// overflowing the stack.
const MAX_DEPTH: usize = 256;

#[derive(Debug, Clone, Default)]
struct Env {
    names: BTreeSet<String>,
}

impl Env {
    fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    fn insert(&mut self, name: String) {
        self.names.insert(name);
    }
}

fn is_callable(signatures: &Signatures, name: &str) -> bool {
    signatures.functions.contains_key(name)
        || signatures.records.contains_key(name)
        || lexicon::is_builtin(name)
}

enum BodyItem {
    Doc(String),
    Example(ast::Example),
    Let {
        name: String,
        name_span: ast::Span,
        value: ast::Expr,
    },
    Do(ast::Expr),
    Result(ast::Expr),
}

impl<'a> Parser<'a> {
    fn parse_bodies(&mut self) {
        let mut functions = std::mem::take(&mut self.ast.fns);
        for function in &mut functions {
            let lines = self.bodies.get(&function.name).cloned().unwrap_or_default();
            self.parse_function_body(function, &lines);
        }
        self.ast.fns = functions;
    }

    fn parse_function_body(&mut self, function: &mut ast::Fn, lines: &[Line]) {
        let mut env = Env::default();
        for param in &function.params {
            if env.contains(&param.name) || is_callable(&self.signatures, &param.name) {
                self.diagnostics.push(*t005(self.file, param.span));
            }
            // Insert even after T005 so later uses resolve as a variable.
            env.insert(param.name.clone());
        }

        let mut result_seen = false;
        let mut last_line_failed = false;
        for line in lines {
            let first = match line.tokens.first() {
                Some(first) => first,
                None => continue,
            };

            if result_seen {
                self.diagnostics
                    .push(*p001(self.file, first, "line after result"));
                last_line_failed = true;
                continue;
            }

            let item = match parse_body_line(self.file, line, &env, &self.signatures) {
                Ok(item) => item,
                Err(diagnostic) => {
                    self.diagnostics.push(*diagnostic);
                    last_line_failed = true;
                    continue;
                }
            };
            last_line_failed = false;

            match item {
                BodyItem::Doc(doc) => {
                    if function.doc.is_some() {
                        self.diagnostics
                            .push(*p001(self.file, first, "duplicate doc"));
                    } else {
                        function.doc = Some(doc);
                    }
                }
                BodyItem::Example(example) => function.examples.push(example),
                BodyItem::Let {
                    name,
                    name_span,
                    value,
                } => {
                    if env.contains(&name) || is_callable(&self.signatures, &name) {
                        self.diagnostics.push(*t005(self.file, name_span));
                    }
                    // Insert even after T005 so later uses resolve as a variable.
                    env.insert(name.clone());
                    function.stmts.push(ast::Stmt {
                        id: NodeId::UNASSIGNED,
                        span: span_of(first),
                        kind: ast::StmtKind::Let { name, value },
                    });
                }
                BodyItem::Do(value) => function.stmts.push(ast::Stmt {
                    id: NodeId::UNASSIGNED,
                    span: span_of(first),
                    kind: ast::StmtKind::Do { value },
                }),
                BodyItem::Result(expr) => {
                    function.result = expr;
                    result_seen = true;
                }
            }
        }

        if !result_seen && !last_line_failed {
            let (line, col) = match lines.last() {
                Some(last) => (last.lineno, eol_col(last)),
                None => match self
                    .lines
                    .iter()
                    .find(|source_line| source_line.lineno == function.span.line)
                {
                    Some(header) => (header.lineno, eol_col(header)),
                    None => (function.span.line, function.span.col),
                },
            };
            self.diagnostics.push(Diagnostic::error(
                "P001",
                self.file,
                line,
                col,
                "missing result",
            ));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKeyword {
    Doc,
    Example,
    Let,
    Do,
}

fn body_keyword(token: &Token) -> Option<BodyKeyword> {
    if is_keyword(token, "intent") {
        Some(BodyKeyword::Doc)
    } else if is_keyword(token, "example") {
        Some(BodyKeyword::Example)
    } else if is_keyword(token, "bind") {
        Some(BodyKeyword::Let)
    } else if is_keyword(token, "effect") {
        Some(BodyKeyword::Do)
    } else {
        None
    }
}

fn parse_body_line(
    file: &str,
    line: &Line,
    env: &Env,
    signatures: &Signatures,
) -> PResult<BodyItem> {
    let tokens = &line.tokens;
    match body_keyword(&tokens[0]) {
        Some(BodyKeyword::Doc) => parse_doc(file, line),
        Some(BodyKeyword::Example) => parse_example(file, line, signatures),
        Some(BodyKeyword::Let) => parse_let(file, line, env, signatures),
        Some(BodyKeyword::Do) => parse_do(file, line, env, signatures),
        None => {
            let value = parse_expr_all(file, line, tokens, env, signatures)?;
            Ok(BodyItem::Result(value))
        }
    }
}

fn parse_doc(file: &str, line: &Line) -> PResult<BodyItem> {
    let tokens = &line.tokens;
    let Some(token) = tokens.get(1) else {
        return Err(p001_eol(file, line, "expected string"));
    };
    let TokenKind::Str(value) = &token.kind else {
        return Err(p001(file, token, "expected string"));
    };
    if let Some(extra) = tokens.get(2) {
        return Err(p001(file, extra, "unexpected token"));
    }
    Ok(BodyItem::Doc(value.clone()))
}

fn parse_example(file: &str, line: &Line, signatures: &Signatures) -> PResult<BodyItem> {
    let tokens = &line.tokens;
    let ex_token = &tokens[0];
    let equals: Vec<usize> = tokens
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, token)| matches!(token.kind, TokenKind::Eq))
        .map(|(index, _)| index)
        .collect();

    match equals.as_slice() {
        [] => Err(p001_eol(file, line, "expected =")),
        [_, extra, ..] => Err(p001(file, &tokens[*extra], "unexpected =")),
        [eq] => {
            let lhs_tokens = &tokens[1..*eq];
            let rhs_tokens = &tokens[eq + 1..];
            if lhs_tokens.is_empty() || rhs_tokens.is_empty() {
                return Err(p001(file, &tokens[*eq], "expected expression"));
            }
            let globals = Env::default();
            let lhs = parse_expr_all(file, line, lhs_tokens, &globals, signatures)?;
            let rhs = parse_expr_all(file, line, rhs_tokens, &globals, signatures)?;
            Ok(BodyItem::Example(ast::Example {
                id: NodeId::UNASSIGNED,
                span: span_of(ex_token),
                lhs,
                rhs,
            }))
        }
    }
}

fn parse_let(file: &str, line: &Line, env: &Env, signatures: &Signatures) -> PResult<BodyItem> {
    let tokens = &line.tokens;
    let Some(name_token) = tokens.get(1) else {
        return Err(p001_eol(file, line, "expected binding name"));
    };
    let TokenKind::Ident(name) = &name_token.kind else {
        return Err(p001(file, name_token, "expected binding name"));
    };
    let value_tokens = &tokens[2..];
    if value_tokens.is_empty() {
        return Err(p001_eol(file, line, "expected expression"));
    }
    let value = parse_expr_all(file, line, value_tokens, env, signatures)?;
    Ok(BodyItem::Let {
        name: name.clone(),
        name_span: span_of(name_token),
        value,
    })
}

fn parse_do(file: &str, line: &Line, env: &Env, signatures: &Signatures) -> PResult<BodyItem> {
    let value_tokens = &line.tokens[1..];
    if value_tokens.is_empty() {
        return Err(p001_eol(file, line, "expected expression"));
    }
    let value = parse_expr_all(file, line, value_tokens, env, signatures)?;
    Ok(BodyItem::Do(value))
}

fn parse_expr_all(
    file: &str,
    line: &Line,
    tokens: &[Token],
    env: &Env,
    signatures: &Signatures,
) -> PResult<ast::Expr> {
    if tokens.is_empty() {
        return Err(p001_eol(file, line, "expected expression"));
    }
    let ctx = ExprCtx {
        file,
        line,
        tokens,
        signatures,
    };
    let (expr, next) = ctx.parse(0, env, 0)?;
    if next < tokens.len() {
        let leftover = &tokens[next];
        if !can_start_expr(leftover) {
            return Err(p001(file, leftover, "unexpected token"));
        }
        return match ctx.head_arity(&tokens[0], env) {
            Some(HeadArity {
                arity: 0,
                zero_call: true,
            }) => {
                let extras = ctx.count_extra_expressions(next, env)?;
                Err(p002(file, &tokens[0], 0, extras))
            }
            Some(HeadArity { arity: 0, .. }) => Err(p001(file, leftover, "unexpected token")),
            Some(HeadArity { arity, .. }) => {
                let extras = ctx.count_extra_expressions(next, env)?;
                Err(p002(file, &tokens[0], arity, arity + extras))
            }
            None => Err(p001(file, leftover, "unexpected token")),
        };
    }
    Ok(expr)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeadArity {
    arity: usize,
    zero_call: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameResolution {
    Var,
    Call { kind: ast::CalleeKind, arity: usize },
}

struct ExprCtx<'a> {
    file: &'a str,
    line: &'a Line,
    tokens: &'a [Token],
    signatures: &'a Signatures,
}

impl<'a> ExprCtx<'a> {
    fn parse(&self, pos: usize, env: &Env, depth: usize) -> PResult<(ast::Expr, usize)> {
        if depth > MAX_DEPTH {
            return Err(self.diagnostic_at(pos, "expression too deep"));
        }
        let token = match self.tokens.get(pos) {
            Some(token) => token,
            None => return Err(p001_eol(self.file, self.line, "expected expression")),
        };
        let span = span_of(token);

        match &token.kind {
            TokenKind::Int(text) => {
                let value = text
                    .parse::<i64>()
                    .map_err(|_| p001(self.file, token, "integer literal out of range"))?;
                Ok((
                    ast::Expr {
                        id: NodeId::UNASSIGNED,
                        span,
                        kind: ast::ExprKind::Int { value },
                    },
                    pos + 1,
                ))
            }
            TokenKind::Float(text) => Ok((
                ast::Expr {
                    id: NodeId::UNASSIGNED,
                    span,
                    kind: ast::ExprKind::Float {
                        value: text.clone(),
                    },
                },
                pos + 1,
            )),
            TokenKind::Str(value) => Ok((
                ast::Expr {
                    id: NodeId::UNASSIGNED,
                    span,
                    kind: ast::ExprKind::Str {
                        value: value.clone(),
                    },
                },
                pos + 1,
            )),
            TokenKind::Keyword(name) => self.parse_keyword(pos, token, name, env, depth, span),
            TokenKind::Ident(name) => self.parse_ident(pos, token, name, env, depth, span),
            TokenKind::FnRef(name) => {
                if !is_callable(self.signatures, name) {
                    return Err(p003(self.file, token, name));
                }
                Ok((
                    ast::Expr {
                        id: NodeId::UNASSIGNED,
                        span,
                        kind: ast::ExprKind::FnRef { name: name.clone() },
                    },
                    pos + 1,
                ))
            }
            TokenKind::Op(name) => match binop_from_spelling(name) {
                Some(op) => self.parse_binop(pos, op, env, depth, span),
                None => Err(p001(self.file, token, "unexpected token")),
            },
            TokenKind::Field(name) => {
                let (args, next) = self.parse_head_args(pos, env, depth)?;
                let mut args = args.into_iter();
                Ok((
                    ast::Expr {
                        id: NodeId::UNASSIGNED,
                        span,
                        kind: ast::ExprKind::Field {
                            name: name.clone(),
                            value: Box::new(args.next().expect("one field argument")),
                        },
                    },
                    next,
                ))
            }
            TokenKind::LBracket => self.parse_list(pos, env, depth, span),
            _ => Err(p001(self.file, token, "expected expression")),
        }
    }

    fn parse_keyword(
        &self,
        pos: usize,
        token: &Token,
        name: &str,
        env: &Env,
        depth: usize,
        span: ast::Span,
    ) -> PResult<(ast::Expr, usize)> {
        if is_keyword(token, "true") {
            Ok((self.bool_expr(true, span), pos + 1))
        } else if is_keyword(token, "false") {
            Ok((self.bool_expr(false, span), pos + 1))
        } else if is_keyword(token, "unitval") {
            Ok((
                ast::Expr {
                    id: NodeId::UNASSIGNED,
                    span,
                    kind: ast::ExprKind::Nil,
                },
                pos + 1,
            ))
        } else if is_keyword(token, "cond") {
            let (args, next) = self.parse_head_args(pos, env, depth)?;
            let mut args = args.into_iter();
            Ok((
                ast::Expr {
                    id: NodeId::UNASSIGNED,
                    span,
                    kind: ast::ExprKind::If {
                        cond: Box::new(args.next().expect("if condition")),
                        then_: Box::new(args.next().expect("if then")),
                        else_: Box::new(args.next().expect("if else")),
                    },
                },
                next,
            ))
        } else if is_keyword(token, "default") {
            let (args, next) = self.parse_head_args(pos, env, depth)?;
            let mut args = args.into_iter();
            Ok((
                ast::Expr {
                    id: NodeId::UNASSIGNED,
                    span,
                    kind: ast::ExprKind::Else {
                        value: Box::new(args.next().expect("else value")),
                        default: Box::new(args.next().expect("else default")),
                    },
                },
                next,
            ))
        } else if is_keyword(token, "optsome") {
            self.parse_unary(pos, env, depth, span, |value| ast::ExprKind::Some {
                value: Box::new(value),
            })
        } else if is_keyword(token, "resok") {
            self.parse_unary(pos, env, depth, span, |value| ast::ExprKind::Ok {
                value: Box::new(value),
            })
        } else if is_keyword(token, "reserr") {
            self.parse_unary(pos, env, depth, span, |value| ast::ExprKind::Err {
                value: Box::new(value),
            })
        } else if is_keyword(token, "propagate") {
            self.parse_unary(pos, env, depth, span, |value| ast::ExprKind::Try {
                value: Box::new(value),
            })
        } else if is_keyword(token, "isome") {
            self.parse_unary(pos, env, depth, span, |value| ast::ExprKind::Has {
                value: Box::new(value),
            })
        } else if is_keyword(token, "isokay") {
            self.parse_unary(pos, env, depth, span, |value| ast::ExprKind::IsOk {
                value: Box::new(value),
            })
        } else if is_keyword(token, "emptylist") {
            self.parse_type_arg(pos, env, depth, span, false)
        } else if is_keyword(token, "optnone") {
            self.parse_type_arg(pos, env, depth, span, true)
        } else if lexicon::is_binop(name) {
            match binop_from_spelling(name) {
                Some(op) => self.parse_binop(pos, op, env, depth, span),
                None => Err(p001(self.file, token, "unexpected token")),
            }
        } else if lexicon::is_unop(name) {
            match unop_from_spelling(name) {
                Some(op) => self.parse_unop(pos, op, env, depth, span),
                None => Err(p001(self.file, token, "unexpected token")),
            }
        } else {
            Err(p001(self.file, token, "unexpected token"))
        }
    }

    fn parse_ident(
        &self,
        pos: usize,
        token: &Token,
        name: &str,
        env: &Env,
        depth: usize,
        span: ast::Span,
    ) -> PResult<(ast::Expr, usize)> {
        match self.resolve_name(name, env) {
            None => Err(p003(self.file, token, name)),
            Some(NameResolution::Var) => Ok((
                ast::Expr {
                    id: NodeId::UNASSIGNED,
                    span,
                    kind: ast::ExprKind::Var {
                        name: name.to_string(),
                    },
                },
                pos + 1,
            )),
            Some(NameResolution::Call { kind, arity }) => {
                self.parse_call(pos, name, kind, arity, env, depth)
            }
        }
    }

    fn resolve_name(&self, name: &str, env: &Env) -> Option<NameResolution> {
        if env.contains(name) {
            Some(NameResolution::Var)
        } else if let Some(arity) = self.signatures.functions.get(name).copied() {
            Some(NameResolution::Call {
                kind: ast::CalleeKind::Fn,
                arity,
            })
        } else if let Some(arity) = self.signatures.records.get(name).copied() {
            Some(NameResolution::Call {
                kind: ast::CalleeKind::Ctor,
                arity,
            })
        } else {
            builtins::lookup(name).map(|spec| NameResolution::Call {
                kind: ast::CalleeKind::Builtin,
                arity: spec.arity(),
            })
        }
    }

    fn parse_call(
        &self,
        pos: usize,
        name: &str,
        kind: ast::CalleeKind,
        arity: usize,
        env: &Env,
        depth: usize,
    ) -> PResult<(ast::Expr, usize)> {
        let (args, next) = self.parse_args(pos, arity, env, depth)?;
        let span = span_of(&self.tokens[pos]);
        Ok((
            ast::Expr {
                id: NodeId::UNASSIGNED,
                span,
                kind: ast::ExprKind::Call {
                    callee: name.to_string(),
                    callee_kind: kind,
                    args,
                },
            },
            next,
        ))
    }

    fn parse_head_args(
        &self,
        pos: usize,
        env: &Env,
        depth: usize,
    ) -> PResult<(Vec<ast::Expr>, usize)> {
        let head = self
            .head_arity(&self.tokens[pos], env)
            .expect("fixed-arity expression head");
        self.parse_args(pos, head.arity, env, depth)
    }

    fn parse_args(
        &self,
        pos: usize,
        count: usize,
        env: &Env,
        depth: usize,
    ) -> PResult<(Vec<ast::Expr>, usize)> {
        let head = &self.tokens[pos];
        let mut args = Vec::with_capacity(count);
        let mut index = pos + 1;
        for _ in 0..count {
            if index >= self.tokens.len() || matches!(&self.tokens[index].kind, TokenKind::RBracket)
            {
                return Err(p002(self.file, head, count, args.len()));
            }
            let (arg, next) = self.parse(index, env, depth + 1)?;
            args.push(arg);
            index = next;
        }
        Ok((args, index))
    }

    fn parse_binop(
        &self,
        pos: usize,
        op: ast::BinOp,
        env: &Env,
        depth: usize,
        span: ast::Span,
    ) -> PResult<(ast::Expr, usize)> {
        let (args, next) = self.parse_head_args(pos, env, depth)?;
        let mut args = args.into_iter();
        Ok((
            ast::Expr {
                id: NodeId::UNASSIGNED,
                span,
                kind: ast::ExprKind::BinOp {
                    op,
                    lhs: Box::new(args.next().expect("binop lhs")),
                    rhs: Box::new(args.next().expect("binop rhs")),
                },
            },
            next,
        ))
    }

    fn parse_unop(
        &self,
        pos: usize,
        op: ast::UnOp,
        env: &Env,
        depth: usize,
        span: ast::Span,
    ) -> PResult<(ast::Expr, usize)> {
        let (args, next) = self.parse_head_args(pos, env, depth)?;
        Ok((
            ast::Expr {
                id: NodeId::UNASSIGNED,
                span,
                kind: ast::ExprKind::UnOp {
                    op,
                    operand: Box::new(args.into_iter().next().expect("unop operand")),
                },
            },
            next,
        ))
    }

    fn parse_unary(
        &self,
        pos: usize,
        env: &Env,
        depth: usize,
        span: ast::Span,
        make: impl FnOnce(ast::Expr) -> ast::ExprKind,
    ) -> PResult<(ast::Expr, usize)> {
        let (args, next) = self.parse_head_args(pos, env, depth)?;
        Ok((
            ast::Expr {
                id: NodeId::UNASSIGNED,
                span,
                kind: make(args.into_iter().next().expect("one argument")),
            },
            next,
        ))
    }

    fn parse_type_arg(
        &self,
        pos: usize,
        env: &Env,
        _depth: usize,
        span: ast::Span,
        none: bool,
    ) -> PResult<(ast::Expr, usize)> {
        let head = &self.tokens[pos];
        let arity = self
            .head_arity(head, env)
            .expect("type-taking expression head")
            .arity;
        let type_pos = pos + 1;
        if type_pos >= self.tokens.len() {
            return Err(p002(self.file, head, arity, 0));
        }
        let (ty, next) = parse_type_at(self.file, self.line, self.tokens, type_pos, 0)?;
        let kind = if none {
            ast::ExprKind::None { ty }
        } else {
            ast::ExprKind::Empty { ty }
        };
        Ok((
            ast::Expr {
                id: NodeId::UNASSIGNED,
                span,
                kind,
            },
            next,
        ))
    }

    fn parse_list(
        &self,
        pos: usize,
        env: &Env,
        depth: usize,
        span: ast::Span,
    ) -> PResult<(ast::Expr, usize)> {
        let mut elems = Vec::new();
        let mut index = pos + 1;
        let mut closed = false;
        while index < self.tokens.len() {
            if matches!(&self.tokens[index].kind, TokenKind::RBracket) {
                index += 1;
                closed = true;
                break;
            }
            let (elem, next) = self.parse(index, env, depth + 1)?;
            elems.push(elem);
            index = next;
        }
        if !closed {
            return Err(p001_eol(self.file, self.line, "expected ]"));
        }
        if elems.is_empty() {
            return Err(p001(self.file, &self.tokens[pos], "empty list"));
        }
        Ok((
            ast::Expr {
                id: NodeId::UNASSIGNED,
                span,
                kind: ast::ExprKind::List { elems },
            },
            index,
        ))
    }

    fn head_arity(&self, token: &Token, env: &Env) -> Option<HeadArity> {
        match &token.kind {
            TokenKind::Int(_) | TokenKind::Float(_) | TokenKind::Str(_) | TokenKind::FnRef(_) => {
                Some(HeadArity {
                    arity: 0,
                    zero_call: false,
                })
            }
            TokenKind::Keyword(name) => {
                let arity = if is_keyword(token, "true")
                    || is_keyword(token, "false")
                    || is_keyword(token, "unitval")
                {
                    Some(0)
                } else if is_keyword(token, "cond") {
                    Some(3)
                } else if is_keyword(token, "default") {
                    Some(2)
                } else if is_keyword(token, "optsome")
                    || is_keyword(token, "resok")
                    || is_keyword(token, "reserr")
                    || is_keyword(token, "propagate")
                    || is_keyword(token, "isome")
                    || is_keyword(token, "isokay")
                    || is_keyword(token, "emptylist")
                    || is_keyword(token, "optnone")
                {
                    Some(1)
                } else if lexicon::is_binop(name) {
                    Some(2)
                } else if lexicon::is_unop(name) {
                    Some(1)
                } else {
                    None
                };
                arity.map(|arity| HeadArity {
                    arity,
                    zero_call: false,
                })
            }
            TokenKind::Ident(name) => {
                self.resolve_name(name, env)
                    .map(|resolution| match resolution {
                        NameResolution::Var => HeadArity {
                            arity: 0,
                            zero_call: false,
                        },
                        NameResolution::Call { arity, .. } => HeadArity {
                            arity,
                            zero_call: true,
                        },
                    })
            }
            TokenKind::Op(name) => binop_from_spelling(name).map(|_| HeadArity {
                arity: 2,
                zero_call: false,
            }),
            TokenKind::Field(_) => Some(HeadArity {
                arity: 1,
                zero_call: false,
            }),
            _ => None,
        }
    }

    fn count_extra_expressions(&self, start: usize, env: &Env) -> PResult<usize> {
        let mut count = 0;
        let mut index = start;
        while index < self.tokens.len() {
            match self.parse(index, env, 0) {
                Ok((_, next)) if next > index => {
                    count += 1;
                    index = next;
                }
                // Stop at the first leftover that is not an expression: an
                // inflated P002 `found` would hide the real syntax error.
                _ => return Err(self.diagnostic_at(index, "unexpected token")),
            }
        }
        Ok(count)
    }

    fn bool_expr(&self, value: bool, span: ast::Span) -> ast::Expr {
        ast::Expr {
            id: NodeId::UNASSIGNED,
            span,
            kind: ast::ExprKind::Bool { value },
        }
    }

    fn diagnostic_at(&self, pos: usize, msg: &str) -> Box<Diagnostic> {
        match self.tokens.get(pos) {
            Some(token) => p001(self.file, token, msg),
            None => p001_eol(self.file, self.line, msg),
        }
    }
}

fn binop_from_spelling(spelling: &str) -> Option<ast::BinOp> {
    [
        ast::BinOp::Add,
        ast::BinOp::Sub,
        ast::BinOp::Mul,
        ast::BinOp::Div,
        ast::BinOp::Mod,
        ast::BinOp::Eq,
        ast::BinOp::Ne,
        ast::BinOp::Lt,
        ast::BinOp::Le,
        ast::BinOp::Gt,
        ast::BinOp::Ge,
        ast::BinOp::And,
        ast::BinOp::Or,
    ]
    .into_iter()
    .find(|op| op.spelling() == spelling)
}

fn unop_from_spelling(spelling: &str) -> Option<ast::UnOp> {
    [ast::UnOp::Not, ast::UnOp::Neg]
        .into_iter()
        .find(|op| op.spelling() == spelling)
}

fn p002(file: &str, token: &Token, expected: usize, found: usize) -> Box<Diagnostic> {
    Box::new(
        Diagnostic::error(
            "P002",
            file,
            token.line,
            token.col,
            "wrong number of arguments",
        )
        .with_expected_found(expected.to_string(), found.to_string()),
    )
}

fn p003(file: &str, token: &Token, name: &str) -> Box<Diagnostic> {
    let mut diagnostic = Diagnostic::error("P003", file, token.line, token.col, "unknown function");
    diagnostic.found = Some(name.to_string());
    Box::new(diagnostic)
}

fn t005(file: &str, span: ast::Span) -> Box<Diagnostic> {
    Box::new(Diagnostic::error(
        "T005",
        file,
        span.line,
        span.col,
        "shadowed binding",
    ))
}

fn is_keyword(token: &Token, concept: &str) -> bool {
    let spelling = lexicon::keyword(concept).expect("lexicon keyword");
    matches!(&token.kind, TokenKind::Keyword(name) if name == spelling)
}

fn is_reserved_name(name: &str) -> bool {
    lexicon::is_builtin(name) || lexicon::is_type_name(name)
}

fn token_spelling(token: &Token) -> &str {
    match &token.kind {
        TokenKind::Keyword(spelling) | TokenKind::Op(spelling) => spelling,
        _ => token.text.as_str(),
    }
}

fn can_start_expr(token: &Token) -> bool {
    match &token.kind {
        TokenKind::Int(_)
        | TokenKind::Float(_)
        | TokenKind::Str(_)
        | TokenKind::Ident(_)
        | TokenKind::FnRef(_)
        | TokenKind::Op(_)
        | TokenKind::Field(_)
        | TokenKind::LBracket => true,
        TokenKind::Keyword(_) => {
            is_keyword(token, "true")
                || is_keyword(token, "false")
                || is_keyword(token, "unitval")
                || is_keyword(token, "cond")
                || is_keyword(token, "default")
                || is_keyword(token, "optsome")
                || is_keyword(token, "optnone")
                || is_keyword(token, "resok")
                || is_keyword(token, "reserr")
                || is_keyword(token, "propagate")
                || is_keyword(token, "isome")
                || is_keyword(token, "isokay")
                || is_keyword(token, "emptylist")
                || lexicon::is_binop(token_spelling(token))
                || lexicon::is_unop(token_spelling(token))
        }
        _ => false,
    }
}

fn is_type(spelling: &str, concept: &str) -> bool {
    lexicon::type_name(concept) == Some(spelling)
}

fn primitive_kind(name: &str) -> Option<ast::TypeKind> {
    if is_type(name, "int") {
        Some(ast::TypeKind::I64)
    } else if is_type(name, "float") {
        Some(ast::TypeKind::F64)
    } else if is_type(name, "boolean") {
        Some(ast::TypeKind::Bool)
    } else if is_type(name, "string") {
        Some(ast::TypeKind::Str)
    } else if is_type(name, "unit") {
        Some(ast::TypeKind::Unit)
    } else {
        None
    }
}

fn span_of(token: &Token) -> ast::Span {
    ast::Span {
        line: token.line,
        col: token.col,
    }
}

fn eol_col(line: &Line) -> u32 {
    line.tokens
        .last()
        .map(|token| token.col + token.text.len() as u32)
        .unwrap_or(1)
}

fn p001(file: &str, token: &Token, msg: &str) -> Box<Diagnostic> {
    Box::new(Diagnostic::error("P001", file, token.line, token.col, msg))
}

fn p001_eol(file: &str, line: &Line, msg: &str) -> Box<Diagnostic> {
    Box::new(Diagnostic::error(
        "P001",
        file,
        line.lineno,
        eol_col(line),
        msg,
    ))
}

fn duplicate(file: &str, token: &Token) -> Box<Diagnostic> {
    p001(file, token, "duplicate definition")
}

fn parse_use(file: &str, line: &Line) -> PResult<ast::Use> {
    let tokens = &line.tokens;
    let use_token = &tokens[0];

    let cap_token = tokens
        .get(1)
        .ok_or_else(|| p001_eol(file, line, "expected capability"))?;
    let cap_name = match &cap_token.kind {
        TokenKind::CapName(name) => name,
        _ => return Err(p001(file, cap_token, "expected capability")),
    };
    let cap = ast::Capability::from_spelling(cap_name)
        .ok_or_else(|| p001(file, cap_token, "expected capability"))?;

    let mut index = 2;
    let path = match tokens.get(index) {
        Some(token) => match &token.kind {
            TokenKind::Str(value) => {
                index += 1;
                Some(value.clone())
            }
            _ => return Err(p001(file, token, "unexpected token")),
        },
        None => None,
    };

    if let Some(token) = tokens.get(index) {
        return Err(p001(file, token, "unexpected token"));
    }

    Ok(ast::Use {
        id: NodeId::UNASSIGNED,
        span: span_of(use_token),
        cap,
        path,
    })
}

fn parse_typedef(file: &str, line: &Line) -> PResult<ast::TypeDef> {
    let tokens = &line.tokens;
    let type_token = &tokens[0];

    let name_token = tokens
        .get(1)
        .ok_or_else(|| p001_eol(file, line, "expected type name"))?;
    let name = match &name_token.kind {
        TokenKind::Ident(name) => name.clone(),
        _ => return Err(p001(file, name_token, "expected type name")),
    };

    let mut fields = Vec::new();
    let mut field_names = BTreeSet::new();
    let mut index = 2;
    while index < tokens.len() {
        let param_token = &tokens[index];
        let field_name = match &param_token.kind {
            TokenKind::Param(name) => name.clone(),
            _ => return Err(p001(file, param_token, "expected field")),
        };
        if !field_names.insert(field_name.clone()) {
            return Err(p001(file, param_token, "duplicate field"));
        }
        index += 1;

        let start = index;
        while index < tokens.len()
            && !matches!(&tokens[index].kind, TokenKind::Param(_) | TokenKind::Arrow)
        {
            index += 1;
        }
        if start == index {
            return Err(p001(file, param_token, "expected type"));
        }
        let ty = parse_type_slice(file, line, &tokens[start..index])?;

        fields.push(ast::Field {
            id: NodeId::UNASSIGNED,
            span: span_of(param_token),
            name: field_name,
            ty,
        });
    }

    Ok(ast::TypeDef {
        id: NodeId::UNASSIGNED,
        span: span_of(type_token),
        name,
        fields,
    })
}

fn parse_fn(file: &str, line: &Line) -> PResult<ast::Fn> {
    let tokens = &line.tokens;
    let fn_token = &tokens[0];

    let name_token = tokens
        .get(1)
        .ok_or_else(|| p001_eol(file, line, "expected function name"))?;
    let name = match &name_token.kind {
        TokenKind::Ident(name) => name.clone(),
        _ => return Err(p001(file, name_token, "expected function name")),
    };

    let mut params = Vec::new();
    let mut index = 2;
    while let Some(param_token) = tokens.get(index) {
        let param_name = match &param_token.kind {
            TokenKind::Param(name) => name.clone(),
            _ => break,
        };
        index += 1;

        let start = index;
        while index < tokens.len()
            && !matches!(&tokens[index].kind, TokenKind::Param(_) | TokenKind::Arrow)
        {
            index += 1;
        }
        if start == index {
            return Err(p001(file, param_token, "expected type"));
        }
        let ty = parse_type_slice(file, line, &tokens[start..index])?;

        params.push(ast::Param {
            id: NodeId::UNASSIGNED,
            span: span_of(param_token),
            name: param_name,
            ty,
        });
    }

    let arrow = tokens
        .get(index)
        .ok_or_else(|| p001_eol(file, line, "expected arrow"))?;
    if !matches!(arrow.kind, TokenKind::Arrow) {
        return Err(p001(file, arrow, "expected arrow"));
    }
    index += 1;

    if index >= tokens.len() {
        return Err(p001_eol(file, line, "expected type"));
    }
    let ret = parse_type_slice(file, line, &tokens[index..])?;

    Ok(ast::Fn {
        id: NodeId::UNASSIGNED,
        span: span_of(fn_token),
        name,
        params,
        ret,
        doc: None,
        examples: Vec::new(),
        stmts: Vec::new(),
        // Placeholder: pass 2 (T-007) replaces this with the body's result.
        result: ast::Expr {
            id: NodeId::UNASSIGNED,
            span: ast::Span {
                line: line.lineno,
                col: eol_col(line),
            },
            kind: ast::ExprKind::Nil,
        },
    })
}

fn parse_type_slice(file: &str, line: &Line, tokens: &[Token]) -> PResult<ast::Type> {
    if tokens.is_empty() {
        return Err(p001_eol(file, line, "expected type"));
    }
    let (ty, used) = parse_type_at(file, line, tokens, 0, 0)?;
    if used < tokens.len() {
        return Err(p001(file, &tokens[used], "unexpected token"));
    }
    Ok(ty)
}

fn parse_type_at(
    file: &str,
    line: &Line,
    tokens: &[Token],
    pos: usize,
    depth: usize,
) -> PResult<(ast::Type, usize)> {
    if depth > MAX_DEPTH {
        return Err(match tokens.get(pos) {
            Some(token) => p001(file, token, "type too deep"),
            None => p001_eol(file, line, "type too deep"),
        });
    }
    let token = match tokens.get(pos) {
        Some(token) => token,
        None => return Err(p001_eol(file, line, "expected type")),
    };
    let name = match &token.kind {
        TokenKind::Ident(name) => name,
        _ => return Err(p001(file, token, "expected type")),
    };

    if let Some(kind) = primitive_kind(name) {
        return Ok((
            ast::Type {
                id: NodeId::UNASSIGNED,
                span: span_of(token),
                kind,
            },
            pos + 1,
        ));
    }

    let arity = if is_type(name, "list") || is_type(name, "option") {
        1
    } else if is_type(name, "result") {
        2
    } else {
        return Ok((
            ast::Type {
                id: NodeId::UNASSIGNED,
                span: span_of(token),
                kind: ast::TypeKind::Named { name: name.clone() },
            },
            pos + 1,
        ));
    };

    let mut children: Vec<Box<ast::Type>> = Vec::new();
    let mut index = pos + 1;
    for _ in 0..arity {
        if index >= tokens.len() {
            break;
        }
        let (child, used) = parse_type_at(file, line, tokens, index, depth + 1)?;
        children.push(Box::new(child));
        index = used;
    }
    if children.len() < arity {
        return Err(p002(file, token, arity, children.len()));
    }

    let kind = if is_type(name, "list") {
        ast::TypeKind::List {
            elem: children.remove(0),
        }
    } else if is_type(name, "option") {
        ast::TypeKind::Opt {
            elem: children.remove(0),
        }
    } else {
        let err = children.remove(1);
        let ok = children.remove(0);
        ast::TypeKind::Res { ok, err }
    };

    Ok((
        ast::Type {
            id: NodeId::UNASSIGNED,
            span: span_of(token),
            kind,
        },
        index,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> ast::File {
        let output = parse("t.aph", source);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            output.diagnostics
        );
        output.file.expect("valid source has an AST")
    }

    fn errors(source: &str) -> Vec<Diagnostic> {
        let output = parse("t.aph", source);
        assert!(output.file.is_none(), "invalid source has no AST");
        output.diagnostics
    }

    fn one_error(source: &str) -> Diagnostic {
        let found = errors(source);
        assert_eq!(found.len(), 1, "expected one diagnostic: {found:?}");
        found[0].clone()
    }

    #[test]
    fn capabilities_round_trip_through_their_spelling() {
        for name in lexicon::CAPABILITIES {
            let cap = ast::Capability::from_spelling(name).expect("known capability");
            assert_eq!(cap.spelling(), *name);
        }
        assert_eq!(ast::Capability::from_spelling("fs.exec"), None);
    }

    #[test]
    fn use_headers_are_collected_and_canonicalized() {
        let file = parse_ok("use fs.read \"data\"\nuse env\n");
        assert_eq!(file.uses.len(), 2);
        assert_eq!(file.uses[0].cap, ast::Capability::Env);
        assert_eq!(file.uses[0].path, None);
        assert_eq!(file.uses[1].cap, ast::Capability::FsRead);
        assert_eq!(file.uses[1].path.as_deref(), Some("data"));
    }

    #[test]
    fn use_path_is_decoded() {
        let file = parse_ok("use fs.read \"a\\tb\"\n");
        assert_eq!(file.uses[0].path.as_deref(), Some("a\tb"));
    }

    #[test]
    fn use_errors() {
        let diagnostic = one_error("use");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 4));

        let diagnostic = one_error("use bogus");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 5));

        let diagnostic = one_error("use fs.read 5");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 13));

        let diagnostic = one_error("use fs.read \"a\" extra");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 17));
    }

    #[test]
    fn type_headers_collect_fields() {
        let file = parse_ok("type point x:f64 y:f64\n");
        assert_eq!(file.types.len(), 1);
        let point = &file.types[0];
        assert_eq!(point.name, "point");
        assert_eq!(point.fields.len(), 2);
        assert_eq!(point.fields[0].name, "x");
        assert_eq!(point.fields[0].ty.kind, ast::TypeKind::F64);
        assert_eq!(point.fields[1].name, "y");
        assert_eq!(point.fields[1].ty.kind, ast::TypeKind::F64);
    }

    #[test]
    fn type_with_no_fields_is_valid() {
        let file = parse_ok("type nothing\n");
        assert_eq!(file.types[0].fields.len(), 0);
    }

    #[test]
    fn nested_types_are_prefix_with_fixed_arity() {
        let file = parse_ok("type a v:res list i64 str\ntype b v:list opt str\n");
        // `File::finish` sorts types by name.
        let a = &file.types[0];
        assert_eq!(a.name, "a");
        let ast::TypeKind::Res { ok, err } = &a.fields[0].ty.kind else {
            panic!("expected res");
        };
        assert_eq!(
            ok.kind,
            ast::TypeKind::List {
                elem: Box::new(ast::Type {
                    id: NodeId::UNASSIGNED,
                    span: ast::Span::default(),
                    kind: ast::TypeKind::I64,
                }),
            }
        );
        assert_eq!(err.kind, ast::TypeKind::Str);

        let b = &file.types[1];
        let ast::TypeKind::List { elem } = &b.fields[0].ty.kind else {
            panic!("expected list");
        };
        let ast::TypeKind::Opt { elem } = &elem.kind else {
            panic!("expected opt");
        };
        assert_eq!(elem.kind, ast::TypeKind::Str);
    }

    #[test]
    fn named_types_are_single_tokens() {
        let file = parse_ok("type alias v:point\n");
        assert_eq!(
            file.types[0].fields[0].ty.kind,
            ast::TypeKind::Named {
                name: "point".to_string()
            }
        );
    }

    #[test]
    fn type_errors() {
        let diagnostic = one_error("type");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 5));

        let diagnostic = one_error("type 1");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 6));

        let diagnostic = one_error("type t x:");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 8));

        let diagnostic = one_error("type t x:list");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 10));
        assert_eq!(diagnostic.expected.as_deref(), Some("1"));
        assert_eq!(diagnostic.found.as_deref(), Some("0"));

        let diagnostic = one_error("type t x:res i64");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 10));
        assert_eq!(diagnostic.expected.as_deref(), Some("2"));
        assert_eq!(diagnostic.found.as_deref(), Some("1"));

        let diagnostic = one_error("type t x:i64 extra");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 14));

        let diagnostic = one_error("type t x:i64 -> i64");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 14));

        let diagnostic = one_error("type t x 1");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 8));

        let diagnostic = one_error("type t x:i64\ntype t y:i64\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "duplicate definition");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 6));
    }

    #[test]
    fn fn_headers_collect_params_and_return_type() {
        let file = parse_ok("fn add a:i64 b:i64 -> i64\n + a b\n");
        let function = &file.fns[0];
        assert_eq!(function.name, "add");
        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].name, "a");
        assert_eq!(function.params[0].ty.kind, ast::TypeKind::I64);
        assert_eq!(function.params[1].name, "b");
        assert_eq!(function.params[1].ty.kind, ast::TypeKind::I64);
        assert_eq!(function.ret.kind, ast::TypeKind::I64);
        assert!(matches!(
            function.result.kind,
            ast::ExprKind::BinOp {
                op: ast::BinOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn fn_with_no_params_and_zero_arity_record() {
        let file = parse_ok("fn main -> unit\n nil\ntype nothing\n");
        assert_eq!(file.fns[0].params.len(), 0);
        assert_eq!(file.fns[0].ret.kind, ast::TypeKind::Unit);
        assert_eq!(file.types[0].fields.len(), 0);
    }

    #[test]
    fn fn_errors() {
        let diagnostic = one_error("fn");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 3));

        let diagnostic = one_error("fn -> i64");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 4));

        let diagnostic = one_error("fn f x:i64");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 11));

        let diagnostic = one_error("fn f ->");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 8));

        let diagnostic = one_error("fn f x:");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 6));

        let diagnostic = one_error("fn f x:list -> i64");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 8));
        assert_eq!(diagnostic.expected.as_deref(), Some("1"));
        assert_eq!(diagnostic.found.as_deref(), Some("0"));

        let diagnostic = one_error("fn f -> i64 extra");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 13));

        let diagnostic = one_error("fn f i64 -> i64");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 6));

        let diagnostic = one_error("fn f -> i64\n 0\nfn f -> i64\n 0\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "duplicate definition");
        assert_eq!((diagnostic.line, diagnostic.col), (3, 4));
    }

    #[test]
    fn body_lines_outside_a_function_are_errors() {
        for source in [
            " 1\n",
            "use env\n 1\n",
            "type t\n 1\n",
            "fn f -> i64\n 0\ntype t\n 1\n",
        ] {
            let diagnostic = one_error(source);
            assert_eq!(diagnostic.code, "P001");
            assert_eq!(diagnostic.msg, "body line outside function");
            assert_eq!(diagnostic.col, 2, "source {source:?}");
        }
    }

    #[test]
    fn a_bad_header_skips_its_body_lines_and_parsing_continues() {
        let (lines, lex_diagnostics) = lexer::lex("t.aph", "fn f i64\n 1\nfn g -> i64\n 2\n");
        assert!(lex_diagnostics.is_empty());

        let mut parser = Parser::new("t.aph", &lines);
        parser.collect_headers();

        assert_eq!(parser.diagnostics.len(), 1);
        assert_eq!(parser.diagnostics[0].code, "P001");
        assert_eq!(parser.ast.fns.len(), 1);
        assert_eq!(parser.ast.fns[0].name, "g");
        assert_eq!(parser.bodies.get("g").map(Vec::len), Some(1));
        assert!(!parser.bodies.contains_key("f"));
    }

    #[test]
    fn signatures_and_record_arities_are_stored_for_pass_two() {
        let (lines, lex_diagnostics) = lexer::lex(
            "t.aph",
            "type point x:f64 y:f64\nfn add a:i64 b:i64 -> i64\n",
        );
        assert!(lex_diagnostics.is_empty());

        let mut parser = Parser::new("t.aph", &lines);
        parser.collect_headers();

        assert_eq!(parser.signatures.functions.get("add"), Some(&2));
        assert_eq!(parser.signatures.records.get("point"), Some(&2));
        assert_eq!(parser.bodies.get("add").map(Vec::len), Some(0));
    }

    #[test]
    fn spec_examples_parse_headers_and_signatures() {
        let source = r#"fn add a:i64 b:i64 -> i64
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
"#;
        let (lines, lex_diagnostics) = lexer::lex("t.aph", source);
        assert!(lex_diagnostics.is_empty());
        let mut parser = Parser::new("t.aph", &lines);
        parser.collect_headers();
        assert!(parser.diagnostics.is_empty(), "{:?}", parser.diagnostics);

        assert_eq!(parser.signatures.functions.get("add"), Some(&2));
        assert_eq!(parser.signatures.functions.get("fib"), Some(&1));
        assert_eq!(parser.signatures.functions.get("total"), Some(&1));
        assert_eq!(parser.signatures.functions.get("main"), Some(&0));
        assert_eq!(parser.bodies.get("add").map(Vec::len), Some(2));
        assert_eq!(parser.bodies.get("fib").map(Vec::len), Some(4));
        assert_eq!(parser.bodies.get("total").map(Vec::len), Some(3));
        assert_eq!(parser.bodies.get("main").map(Vec::len), Some(1));

        let file = parse_ok(source);
        let names: Vec<&str> = file.fns.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["add", "fib", "total", "main"]);
        let total = &file.fns[2];
        assert_eq!(
            total.params[0].ty.kind,
            ast::TypeKind::List {
                elem: Box::new(ast::Type {
                    id: NodeId::UNASSIGNED,
                    span: ast::Span::default(),
                    kind: ast::TypeKind::I64,
                }),
            }
        );
    }

    #[test]
    fn spec_capability_example_parses() {
        let source = r#"use fs.read "data"
fn words s:str -> i64
 ex words "a b c" = 3
 len split trim s " "
fn main -> res unit str
 let text try read "data/in.txt"
 do print show words text
 ok nil
"#;
        let file = parse_ok(source);
        assert_eq!(file.uses.len(), 1);
        assert_eq!(file.uses[0].cap, ast::Capability::FsRead);
        assert_eq!(file.uses[0].path.as_deref(), Some("data"));
        let words = &file.fns[0];
        assert_eq!(words.name, "words");
        assert_eq!(words.params.len(), 1);
        assert_eq!(words.ret.kind, ast::TypeKind::I64);
        let main = &file.fns[1];
        let ast::TypeKind::Res { ok, err } = &main.ret.kind else {
            panic!("expected res");
        };
        assert_eq!(ok.kind, ast::TypeKind::Unit);
        assert_eq!(err.kind, ast::TypeKind::Str);
    }

    #[test]
    fn lexical_errors_stop_parsing() {
        let output = parse("t.aph", "Foo\n");
        assert!(output.file.is_none());
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].code, "L002");
    }

    #[test]
    fn empty_source_is_a_valid_empty_file() {
        let file = parse_ok("");
        assert!(file.uses.is_empty());
        assert!(file.types.is_empty());
        assert!(file.fns.is_empty());
        assert_eq!(file.id, NodeId(0));
        assert_eq!(file.uses.len(), 0);
    }

    #[test]
    fn every_lexicon_type_has_an_arity() {
        for spelling in lexicon::type_names() {
            let known = primitive_kind(spelling).is_some()
                || is_type(spelling, "list")
                || is_type(spelling, "option")
                || is_type(spelling, "result");
            assert!(known, "unhandled type name: {spelling}");
        }
    }
    fn function<'a>(file: &'a ast::File, name: &str) -> &'a ast::Fn {
        file.fns
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing function {name}"))
    }

    fn expr_kind(expr: &str) -> ast::ExprKind {
        let source = format!("fn f -> i64\n {expr}\n");
        let file = parse_ok(&source);
        function(&file, "f").result.kind.clone()
    }

    #[test]
    fn literal_expressions() {
        assert_eq!(expr_kind("42"), ast::ExprKind::Int { value: 42 });
        assert_eq!(expr_kind("-5"), ast::ExprKind::Int { value: -5 });
        assert_eq!(
            expr_kind("3.14"),
            ast::ExprKind::Float {
                value: "3.14".to_string()
            }
        );
        assert_eq!(
            expr_kind("1.00"),
            ast::ExprKind::Float {
                value: "1.00".to_string()
            }
        );
        assert_eq!(
            expr_kind("\"hi\""),
            ast::ExprKind::Str {
                value: "hi".to_string()
            }
        );
        assert_eq!(expr_kind("true"), ast::ExprKind::Bool { value: true });
        assert_eq!(expr_kind("false"), ast::ExprKind::Bool { value: false });
        assert_eq!(expr_kind("nil"), ast::ExprKind::Nil);
    }

    #[test]
    fn integer_literals_out_of_range_are_p001() {
        let diagnostic = one_error("fn f -> i64\n 999999999999999999999999999999\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "integer literal out of range");
    }

    #[test]
    fn identifiers_resolve_to_variables_first() {
        let source = "fn f x:i64 -> i64\n x\n";
        let file = parse_ok(source);
        assert_eq!(
            function(&file, "f").result.kind,
            ast::ExprKind::Var {
                name: "x".to_string()
            }
        );
    }

    #[test]
    fn calls_resolve_user_functions_constructors_and_builtins() {
        let file = parse_ok("fn g a:i64 -> i64\n a\nfn f -> i64\n g 1\n");
        let ast::ExprKind::Call {
            callee,
            callee_kind,
            args,
        } = function(&file, "f").result.kind.clone()
        else {
            panic!("expected user call");
        };
        assert_eq!(callee, "g");
        assert_eq!(callee_kind, ast::CalleeKind::Fn);
        assert_eq!(args.len(), 1);

        let file = parse_ok("type point x:i64 y:i64\nfn f -> point\n point 1 2\n");
        let ast::ExprKind::Call {
            callee,
            callee_kind,
            args,
        } = function(&file, "f").result.kind.clone()
        else {
            panic!("expected ctor call");
        };
        assert_eq!(callee, "point");
        assert_eq!(callee_kind, ast::CalleeKind::Ctor);
        assert_eq!(args.len(), 2);

        let kind = expr_kind("len [1 2 3]");
        let ast::ExprKind::Call {
            callee,
            callee_kind,
            args,
        } = kind
        else {
            panic!("expected builtin call");
        };
        assert_eq!(callee, "len");
        assert_eq!(callee_kind, ast::CalleeKind::Builtin);
        assert_eq!(args.len(), 1);

        let kind = expr_kind("args");
        let ast::ExprKind::Call {
            callee,
            callee_kind,
            args,
        } = kind
        else {
            panic!("expected zero-arity builtin call");
        };
        assert_eq!(callee, "args");
        assert_eq!(callee_kind, ast::CalleeKind::Builtin);
        assert!(args.is_empty());
    }

    #[test]
    fn fnrefs_require_a_known_callable() {
        let file = parse_ok("fn g a:i64 -> i64\n a\nfn f -> i64\n @g\n");
        assert_eq!(
            function(&file, "f").result.kind,
            ast::ExprKind::FnRef {
                name: "g".to_string()
            }
        );

        let diagnostic = one_error("fn f -> i64\n @nope\n");
        assert_eq!(diagnostic.code, "P003");
        assert_eq!(diagnostic.found.as_deref(), Some("nope"));
    }

    #[test]
    fn symbolic_and_keyword_operators() {
        assert!(matches!(
            expr_kind("+ 1 2"),
            ast::ExprKind::BinOp {
                op: ast::BinOp::Add,
                ..
            }
        ));
        let ast::ExprKind::BinOp { op, lhs, rhs } = expr_kind("- 5 3") else {
            panic!("expected subtraction");
        };
        assert_eq!(op, ast::BinOp::Sub);
        assert_eq!(lhs.kind, ast::ExprKind::Int { value: 5 });
        assert_eq!(rhs.kind, ast::ExprKind::Int { value: 3 });
        assert_eq!(expr_kind("-5"), ast::ExprKind::Int { value: -5 });

        assert!(matches!(
            expr_kind("and true false"),
            ast::ExprKind::BinOp {
                op: ast::BinOp::And,
                ..
            }
        ));
        assert!(matches!(
            expr_kind("not true"),
            ast::ExprKind::UnOp {
                op: ast::UnOp::Not,
                ..
            }
        ));
        assert!(matches!(
            expr_kind("neg 5"),
            ast::ExprKind::UnOp {
                op: ast::UnOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn control_and_option_result_forms() {
        assert!(matches!(expr_kind("if true 1 2"), ast::ExprKind::If { .. }));
        assert!(matches!(
            expr_kind("else some 1 0"),
            ast::ExprKind::Else { .. }
        ));
        assert!(matches!(expr_kind("some 1"), ast::ExprKind::Some { .. }));
        assert!(matches!(expr_kind("ok 1"), ast::ExprKind::Ok { .. }));
        assert!(matches!(expr_kind("err 1"), ast::ExprKind::Err { .. }));
        assert!(matches!(expr_kind("try 1"), ast::ExprKind::Try { .. }));
        assert!(matches!(expr_kind("has 1"), ast::ExprKind::Has { .. }));
        assert!(matches!(expr_kind("isok 1"), ast::ExprKind::IsOk { .. }));
    }

    #[test]
    fn empty_and_none_take_a_type() {
        let ast::ExprKind::Empty { ty } = expr_kind("empty list i64") else {
            panic!("expected empty");
        };
        assert!(matches!(ty.kind, ast::TypeKind::List { .. }));
        let ast::ExprKind::None { ty } = expr_kind("none opt str") else {
            panic!("expected none");
        };
        assert!(matches!(ty.kind, ast::TypeKind::Opt { .. }));
    }

    #[test]
    fn field_access_is_prefix() {
        let source = "fn f p:point -> i64\n .x p\n";
        let file = parse_ok(source);
        let ast::ExprKind::Field { name, value } = function(&file, "f").result.kind.clone() else {
            panic!("expected field");
        };
        assert_eq!(name, "x");
        assert_eq!(
            value.kind,
            ast::ExprKind::Var {
                name: "p".to_string()
            }
        );
    }

    #[test]
    fn lists_and_nesting() {
        let ast::ExprKind::List { elems } = expr_kind("[1 2 3]") else {
            panic!("expected list");
        };
        assert_eq!(elems.len(), 3);

        let ast::ExprKind::List { elems } = expr_kind("[[1 2] [3]]") else {
            panic!("expected list");
        };
        assert_eq!(elems.len(), 2);
        assert!(matches!(elems[0].kind, ast::ExprKind::List { .. }));
        assert!(matches!(elems[1].kind, ast::ExprKind::List { .. }));
    }

    #[test]
    fn list_errors() {
        let diagnostic = one_error("fn f -> i64\n []\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "empty list");

        let diagnostic = one_error("fn f -> i64\n [1 2\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "expected ]");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 6));
    }

    #[test]
    fn argument_count_errors_are_p002() {
        let diagnostic = one_error("fn f -> i64\n + 1\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 2));
        assert_eq!(diagnostic.expected.as_deref(), Some("2"));
        assert_eq!(diagnostic.found.as_deref(), Some("1"));

        let diagnostic = one_error("fn f -> i64\n len\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!(diagnostic.expected.as_deref(), Some("1"));
        assert_eq!(diagnostic.found.as_deref(), Some("0"));

        let diagnostic = one_error("fn f -> i64\n + 1 2 3\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!(diagnostic.expected.as_deref(), Some("2"));
        assert_eq!(diagnostic.found.as_deref(), Some("3"));

        let diagnostic = one_error("fn f -> i64\n 42 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "unexpected token");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 5));

        let diagnostic = one_error("fn f x:i64 -> i64\n x 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 4));

        let diagnostic = one_error("fn f -> i64\n args 1\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 2));
        assert_eq!(diagnostic.expected.as_deref(), Some("0"));
        assert_eq!(diagnostic.found.as_deref(), Some("1"));

        let diagnostic = one_error("fn f -> i64\n + 1 2 3 4\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!(diagnostic.expected.as_deref(), Some("2"));
        assert_eq!(diagnostic.found.as_deref(), Some("4"));

        // An unparsable leftover stops the extra-expression count: report the
        // real P001 at `]` instead of an inflated P002.
        let diagnostic = one_error("fn f -> i64\n + 1 2 3 ]\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "unexpected token");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 10));

        let diagnostic = one_error("fn f -> i64\n 1 ]\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "unexpected token");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 4));

        let diagnostic = one_error("fn f -> i64\n [+ 1]\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 3));
        assert_eq!(diagnostic.expected.as_deref(), Some("2"));
        assert_eq!(diagnostic.found.as_deref(), Some("1"));
    }

    #[test]
    fn unknown_identifiers_are_p003() {
        let diagnostic = one_error("fn f -> i64\n nope\n");
        assert_eq!(diagnostic.code, "P003");
        assert_eq!(diagnostic.msg, "unknown function");
        assert_eq!(diagnostic.found.as_deref(), Some("nope"));
    }

    #[test]
    fn shadowing_local_bindings_is_t005() {
        let diagnostic = one_error("fn f x:i64 -> i64\n let x 1\n x\n");
        assert_eq!(diagnostic.code, "T005");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 6));

        let diagnostic = one_error("fn f -> i64\n let x 1\n let x 2\n x\n");
        assert_eq!(diagnostic.code, "T005");
        assert_eq!((diagnostic.line, diagnostic.col), (3, 6));
    }

    #[test]
    fn shadowing_a_callable_is_t005_and_inserts_the_binding() {
        // If the binding were not inserted, `len` would be a P002 call.
        let diagnostic = one_error("fn f -> i64\n let len 1\n len\n");
        assert_eq!(diagnostic.code, "T005");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 6));

        let source = "fn g a:i64 -> i64\n a\nfn f -> i64\n let g 1\n g\n";
        let diagnostic = one_error(source);
        assert_eq!(diagnostic.code, "T005");
        assert_eq!((diagnostic.line, diagnostic.col), (4, 6));

        let source = "fn g a:i64 -> i64\n a\nfn f g:i64 -> i64\n g\n";
        let diagnostic = one_error(source);
        assert_eq!(diagnostic.code, "T005");
        assert_eq!((diagnostic.line, diagnostic.col), (3, 6));
    }

    #[test]
    fn every_colliding_param_is_reported() {
        let found = errors("fn f len:i64 len:i64 -> i64\n 0\n");
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|diagnostic| diagnostic.code == "T005"));
        assert_eq!(found[0].line, 1);
        assert_eq!(found[1].line, 1);
    }

    #[test]
    fn reserved_top_level_names_are_p001() {
        for source in [
            "fn len -> i64\n 0\n",
            "fn read -> i64\n 0\n",
            "fn list -> i64\n 0\n",
            "type list\n",
            "type i64\n",
            "type read\n",
        ] {
            let diagnostic = one_error(source);
            assert_eq!(diagnostic.code, "P001", "source {source:?}");
            assert_eq!(diagnostic.msg, "reserved name", "source {source:?}");
        }
    }

    #[test]
    fn duplicate_type_fields_are_p001() {
        let diagnostic = one_error("type t x:i64 x:i64\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "duplicate field");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 14));
    }

    #[test]
    fn duplicate_params_are_t005() {
        let diagnostic = one_error("fn f x:i64 x:i64 -> i64\n x\n");
        assert_eq!(diagnostic.code, "T005");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 12));
    }

    #[test]
    fn examples_split_at_the_single_equals() {
        let file = parse_ok("fn f x:i64 -> i64\n ex f 1 = 1\n x\n");
        let examples = &function(&file, "f").examples;
        assert_eq!(examples.len(), 1);
        assert!(matches!(
            examples[0].lhs.kind,
            ast::ExprKind::Call {
                callee_kind: ast::CalleeKind::Fn,
                ..
            }
        ));
        assert_eq!(examples[0].rhs.kind, ast::ExprKind::Int { value: 1 });

        let diagnostic = one_error("fn f -> i64\n ex f 1\n 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "expected =");

        let diagnostic = one_error("fn f -> i64\n ex f 1 = 1 = 2\n 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "unexpected =");
    }

    #[test]
    fn examples_resolve_against_globals_only() {
        let diagnostic = one_error("fn f x:i64 -> i64\n ex x = 1\n x\n");
        assert_eq!(diagnostic.code, "P003");
        assert_eq!(diagnostic.found.as_deref(), Some("x"));
    }

    #[test]
    fn lets_use_previous_bindings() {
        let source = "fn f -> i64\n let x 1\n let y x\n + y x\n";
        let file = parse_ok(source);
        let function = function(&file, "f");
        assert_eq!(function.stmts.len(), 2);
        let ast::StmtKind::Let { name, value } = &function.stmts[1].kind else {
            panic!("expected second let");
        };
        assert_eq!(name, "y");
        assert_eq!(
            value.kind,
            ast::ExprKind::Var {
                name: "x".to_string()
            }
        );

        let diagnostic = one_error("fn f -> i64\n let x x\n 0\n");
        assert_eq!(diagnostic.code, "P003");
        assert_eq!(diagnostic.found.as_deref(), Some("x"));
    }

    #[test]
    fn lines_after_the_result_are_p001() {
        let diagnostic = one_error("fn f -> i64\n 1\n 2\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "line after result");
        assert_eq!(diagnostic.line, 3);
    }

    #[test]
    fn missing_result_is_p001_at_end_of_line() {
        let diagnostic = one_error("fn f -> i64\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "missing result");
        assert_eq!((diagnostic.line, diagnostic.col), (1, 12));

        let diagnostic = one_error("fn f -> i64\n doc \"x\"\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "missing result");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 9));
    }

    #[test]
    fn doc_lines_are_checked() {
        let diagnostic = one_error("fn f -> i64\n doc \"a\"\n doc \"b\"\n 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "duplicate doc");
        assert_eq!(diagnostic.line, 3);

        let diagnostic = one_error("fn f -> i64\n doc 1\n 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "expected string");

        let diagnostic = one_error("fn f -> i64\n doc \"a\" \"b\"\n 1\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "unexpected token");
    }

    #[test]
    fn doc_examples_and_statements_can_be_in_any_order() {
        let source =
            "fn f a:i64 -> i64\n ex f 0 = 0\n let x 1\n doc \"d\"\n do print \"x\"\n + x a\n";
        let file = parse_ok(source);
        let function = function(&file, "f");
        assert_eq!(function.doc.as_deref(), Some("d"));
        assert_eq!(function.examples.len(), 1);
        assert_eq!(function.stmts.len(), 2);
        assert!(matches!(
            function.result.kind,
            ast::ExprKind::BinOp {
                op: ast::BinOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn fn_and_type_names_collide() {
        let diagnostic = one_error("type t x:i64\nfn t -> i64\n 0\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "duplicate definition");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 4));

        let diagnostic = one_error("fn t -> i64\n 0\ntype t x:i64\n");
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "duplicate definition");
        assert_eq!((diagnostic.line, diagnostic.col), (3, 6));
    }

    #[test]
    fn nesting_beyond_the_limit_is_p001() {
        let mut source = String::from("fn f -> i64\n ");
        source.push_str(&"[".repeat(MAX_DEPTH + 2));
        source.push('1');
        source.push_str(&"]".repeat(MAX_DEPTH + 2));
        source.push('\n');

        let diagnostic = one_error(&source);
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "expression too deep");
    }

    #[test]
    fn type_nesting_beyond_the_limit_is_p001() {
        let mut source = String::from("type t x:");
        source.push_str(&"list ".repeat(6000));
        source.push_str("i64\n");

        let diagnostic = one_error(&source);
        assert_eq!(diagnostic.code, "P001");
        assert_eq!(diagnostic.msg, "type too deep");
    }

    #[test]
    fn a_failed_result_line_does_not_mark_later_lines_as_after_result() {
        let diagnostic = one_error("fn f -> i64\n + 1\n 2\n");
        assert_eq!(diagnostic.code, "P002");
        assert_eq!((diagnostic.line, diagnostic.col), (2, 2));

        let diagnostic = one_error("fn f -> i64\n nope\n 1\n");
        assert_eq!(diagnostic.code, "P003");
        assert_eq!(diagnostic.found.as_deref(), Some("nope"));
    }

    #[test]
    fn spec_programs_parse_fully() {
        let source = r#"fn add a:i64 b:i64 -> i64
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
"#;
        let file = parse_ok(source);
        let add = function(&file, "add");
        assert!(add.doc.is_none());
        assert_eq!(add.examples.len(), 1);
        assert!(add.stmts.is_empty());
        assert!(matches!(
            add.result.kind,
            ast::ExprKind::BinOp {
                op: ast::BinOp::Add,
                ..
            }
        ));

        let fib = function(&file, "fib");
        assert_eq!(fib.doc.as_deref(), Some("nth fibonacci number"));
        assert_eq!(fib.examples.len(), 2);
        assert!(matches!(fib.result.kind, ast::ExprKind::If { .. }));

        let total = function(&file, "total");
        assert_eq!(total.examples.len(), 2);
        let ast::ExprKind::Call {
            callee,
            callee_kind,
            args,
        } = total.result.kind.clone()
        else {
            panic!("expected fold call");
        };
        assert_eq!(callee, "fold");
        assert_eq!(callee_kind, ast::CalleeKind::Builtin);
        assert_eq!(args.len(), 3);
        assert!(matches!(args[0].kind, ast::ExprKind::FnRef { .. }));

        let main = function(&file, "main");
        let ast::ExprKind::Call { callee, args, .. } = main.result.kind.clone() else {
            panic!("expected print call");
        };
        assert_eq!(callee, "print");
        assert_eq!(args.len(), 1);

        let source = r#"use fs.read "data"
fn words s:str -> i64
 ex words "a b c" = 3
 len split trim s " "
fn main -> res unit str
 let text try read "data/in.txt"
 do print show words text
 ok nil
"#;
        let file = parse_ok(source);
        let words = function(&file, "words");
        assert_eq!(words.examples.len(), 1);
        assert!(matches!(words.result.kind, ast::ExprKind::Call { .. }));

        let main = function(&file, "main");
        assert_eq!(main.stmts.len(), 2);
        assert!(matches!(main.stmts[0].kind, ast::StmtKind::Let { .. }));
        assert!(matches!(main.stmts[1].kind, ast::StmtKind::Do { .. }));
        let ast::ExprKind::Ok { value } = &main.result.kind else {
            panic!("expected ok");
        };
        assert_eq!(value.kind, ast::ExprKind::Nil);
    }
}
