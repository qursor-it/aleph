//! Lexer for Aleph source files (SPEC §1-2, `spec/grammar.ebnf`).
//!
//! The lexer is tolerant of non-canonical whitespace so that `aleph fmt` can
//! repair it (ADR-0016). It reports every lexical error it finds as a SPEC §9
//! diagnostic and keeps scanning.

use crate::diag::Diagnostic;
use crate::lexicon;

/// Whether a line starts a top-level item or is a body line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    TopLevel,
    Body,
}

/// One source line with its token sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 1-based line number in the source file.
    pub lineno: u32,
    pub kind: LineKind,
    pub tokens: Vec<Token>,
}

/// One token with its raw spelling and 1-based source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// Raw source text. For [`TokenKind::Str`] this includes the quotes; the
    /// decoded value lives in the token kind.
    pub text: String,
    /// 1-based line number.
    pub line: u32,
    /// 1-based byte column.
    pub col: u32,
}

/// Lexical token classes (SPEC §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// `[a-z][a-z0-9]*` that is not a keyword. Built-in type names also lex as
    /// identifiers; the parser consults [`lexicon`] for type spellings.
    Ident(String),
    Keyword(String),
    Int(String),
    Float(String),
    /// Decoded string value (escapes resolved).
    Str(String),
    /// `@ident`.
    FnRef(String),
    /// `.ident`.
    Field(String),
    /// `ident:` (the type token follows on the next column).
    Param(String),
    LBracket,
    RBracket,
    Eq,
    Arrow,
    /// `|`, separating sum variants (ADR-0034).
    Bar,
    /// Symbolic binary operator; `and`/`or`/`not`/`neg` lex as
    /// [`TokenKind::Keyword`] — use [`lexicon::is_binop`] / [`lexicon::is_unop`].
    Op(String),
    /// Capability name immediately after `use`.
    CapName(String),
}

/// Lex a source file. Returns the lines with their tokens and all diagnostics
/// in source order, sorted by `(line, col)`.
///
/// `lines` is only meaningful when `diagnostics` is empty: after a lexical
/// error the token stream may be incomplete.
pub fn lex(file: &str, source: &str) -> (Vec<Line>, Vec<Diagnostic>) {
    let mut lines = Vec::new();
    let mut diagnostics = Vec::new();

    for (index, text) in source.split('\n').enumerate() {
        // Empty lines and lines made only of spaces are skipped (ADR-0016).
        if text.bytes().all(|byte| byte == b' ') {
            continue;
        }
        let kind = if text.starts_with(' ') {
            LineKind::Body
        } else {
            LineKind::TopLevel
        };
        let start = if kind == LineKind::Body {
            text.bytes().take_while(|byte| *byte == b' ').count()
        } else {
            0
        };

        let mut scanner = LineScanner {
            file,
            text,
            lineno: index as u32 + 1,
            pos: start,
            diagnostics: &mut diagnostics,
        };
        let tokens = scanner.run();
        if !tokens.is_empty() {
            lines.push(Line {
                lineno: scanner.lineno,
                kind,
                tokens,
            });
        }
    }

    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.col));

    (lines, diagnostics)
}

struct LineScanner<'a> {
    file: &'a str,
    text: &'a str,
    lineno: u32,
    pos: usize,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> LineScanner<'a> {
    fn len(&self) -> usize {
        self.text.len()
    }

    fn byte(&self, at: usize) -> u8 {
        self.text.as_bytes()[at]
    }

    fn push_error(&mut self, code: &str, col: usize, msg: &str) {
        self.diagnostics.push(Diagnostic::error(
            code,
            self.file,
            self.lineno,
            col as u32,
            msg,
        ));
    }

    fn token(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            text: self.text[start..end].to_string(),
            line: self.lineno,
            col: start as u32 + 1,
        }
    }

    fn run(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut expect_capname = false;
        let fnref_mark = lexicon::operator("fnref").expect("fnref punctuation");
        let field_mark = lexicon::operator("field").expect("field punctuation");
        let bar_mark = lexicon::operator("bar").expect("bar punctuation");

        while self.pos < self.len() {
            let byte = self.byte(self.pos);
            if byte == b' ' {
                self.pos += 1;
                continue;
            }
            // Outside strings only printable ASCII (0x20..=0x7e) is valid.
            if !(0x20..0x7f).contains(&byte) {
                self.invalid_char();
                continue;
            }

            let want_capname = expect_capname;
            expect_capname = false;

            if byte == b'"' {
                if let Some(token) = self.string() {
                    tokens.push(token);
                }
            } else if byte == b'[' {
                let start = self.pos;
                self.pos += 1;
                tokens.push(self.token(TokenKind::LBracket, start, self.pos));
            } else if byte == b']' {
                let start = self.pos;
                self.pos += 1;
                tokens.push(self.token(TokenKind::RBracket, start, self.pos));
            } else if byte == b'-'
                && self.pos + 1 < self.len()
                && self.byte(self.pos + 1).is_ascii_digit()
            {
                if let Some(token) = self.number() {
                    tokens.push(token);
                }
            } else if self.arrow_at() {
                let start = self.pos;
                self.pos += lexicon::operator("arrow").expect("arrow punctuation").len();
                tokens.push(self.token(TokenKind::Arrow, start, self.pos));
            } else if let Some(op) = self.binary_operator_at() {
                let start = self.pos;
                self.pos += op.len();
                tokens.push(self.token(TokenKind::Op(op.to_string()), start, self.pos));
            } else if byte == b'=' {
                let start = self.pos;
                self.pos += 1;
                tokens.push(self.token(TokenKind::Eq, start, self.pos));
            } else if byte == bar_mark.as_bytes()[0] {
                let start = self.pos;
                self.pos += 1;
                tokens.push(self.token(TokenKind::Bar, start, self.pos));
            } else if byte == fnref_mark.as_bytes()[0] {
                if let Some(token) = self.fnref() {
                    tokens.push(token);
                }
            } else if byte == field_mark.as_bytes()[0] {
                if let Some(token) = self.field() {
                    tokens.push(token);
                }
            } else if byte.is_ascii_digit() {
                if let Some(token) = self.number() {
                    tokens.push(token);
                }
            } else if byte.is_ascii_lowercase() {
                if let Some(token) = self.word(want_capname) {
                    let capability_keyword =
                        lexicon::keyword("capability").expect("capability keyword");
                    if matches!(
                        &token.kind,
                        TokenKind::Keyword(name) if name.as_str() == capability_keyword
                    ) {
                        expect_capname = true;
                    }
                    tokens.push(token);
                }
            } else {
                self.invalid_token();
            }
        }

        tokens
    }

    fn arrow_at(&self) -> bool {
        let arrow = lexicon::operator("arrow").expect("arrow punctuation");
        self.text[self.pos..].starts_with(arrow)
    }

    fn binary_operator_at(&self) -> Option<&'static str> {
        let rest = &self.text[self.pos..];
        lexicon::binary_operator_names()
            .filter(|op| rest.starts_with(*op))
            .max_by_key(|op| op.len())
    }

    fn capability_end(&self, start: usize, ident_end: usize) -> Option<usize> {
        if ident_end < self.len()
            && self.byte(ident_end) == b'.'
            && ident_end + 1 < self.len()
            && self.byte(ident_end + 1).is_ascii_lowercase()
        {
            let mut end = ident_end + 1;
            while end < self.len()
                && (self.byte(end).is_ascii_lowercase() || self.byte(end).is_ascii_digit())
            {
                end += 1;
            }
            if lexicon::is_capability(&self.text[start..end]) {
                return Some(end);
            }
        }
        if lexicon::is_capability(&self.text[start..ident_end]) {
            Some(ident_end)
        } else {
            None
        }
    }

    fn word(&mut self, want_capname: bool) -> Option<Token> {
        let start = self.pos;
        let mut end = start;
        while end < self.len()
            && (self.byte(end).is_ascii_lowercase() || self.byte(end).is_ascii_digit())
        {
            end += 1;
        }

        if want_capname {
            if let Some(cap_end) = self.capability_end(start, end) {
                let name = self.text[start..cap_end].to_string();
                self.pos = cap_end;
                return Some(self.token(TokenKind::CapName(name), start, cap_end));
            }
        }

        let name = self.text[start..end].to_string();
        let is_keyword = lexicon::is_keyword(&name);
        if !is_keyword && end < self.len() && self.byte(end) == b':' {
            self.pos = end + 1;
            return Some(self.token(TokenKind::Param(name), start, end + 1));
        }

        let kind = if is_keyword {
            TokenKind::Keyword(name)
        } else {
            TokenKind::Ident(name)
        };
        self.pos = end;
        Some(self.token(kind, start, end))
    }

    fn number(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut index = start;
        let negative = self.byte(index) == b'-';
        if negative {
            index += 1;
        }
        let digits_start = index;
        while index < self.len() && self.byte(index).is_ascii_digit() {
            index += 1;
        }
        let int_digits = &self.text[digits_start..index];

        let mut is_float = false;
        if index < self.len() && self.byte(index) == b'.' {
            is_float = true;
            index += 1;
            let fraction_start = index;
            while index < self.len() && self.byte(index).is_ascii_digit() {
                index += 1;
            }
            if index == fraction_start {
                let end = self.consume_word_tail(index);
                self.push_error("L002", start + 1, "invalid token");
                self.pos = end;
                return None;
            }
        }

        if index < self.len() && is_word_byte(self.byte(index)) {
            let end = self.consume_word_tail(index);
            self.push_error("L002", start + 1, "invalid token");
            self.pos = end;
            return None;
        }

        let invalid_zero = int_digits.starts_with('0') && (int_digits.len() > 1 || negative);
        if !is_float && (int_digits.is_empty() || invalid_zero) {
            self.push_error("L002", start + 1, "invalid token");
            self.pos = index;
            return None;
        }

        let text = self.text[start..index].to_string();
        let kind = if is_float {
            TokenKind::Float(text)
        } else {
            TokenKind::Int(text)
        };
        self.pos = index;
        Some(self.token(kind, start, index))
    }

    fn string(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut index = start + 1;
        let mut value = String::new();
        let mut closed = false;

        while index < self.len() {
            match self.byte(index) {
                b'"' => {
                    index += 1;
                    closed = true;
                    break;
                }
                b'\\' => {
                    if index + 1 >= self.len() {
                        break;
                    }
                    let escaped = self.text[index + 1..]
                        .chars()
                        .next()
                        .expect("escaped character is present");
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        _ => {
                            value.push('\\');
                            value.push(escaped);
                            self.push_error("L002", index + 1, "invalid token");
                        }
                    }
                    index += 1 + escaped.len_utf8();
                }
                _ => {
                    // Raw control characters (C0, DEL, C1) are not allowed in
                    // strings; only the `\n` and `\t` escapes are (SPEC §2).
                    let ch = self.text[index..]
                        .chars()
                        .next()
                        .expect("character is present");
                    if ch.is_control() {
                        self.push_error("L001", index + 1, "invalid character");
                    }
                    value.push(ch);
                    index += ch.len_utf8();
                }
            }
        }

        if !closed {
            self.push_error("L002", start + 1, "invalid token");
            self.pos = self.len();
            return None;
        }

        self.pos = index;
        Some(self.token(TokenKind::Str(value), start, index))
    }

    fn fnref(&mut self) -> Option<Token> {
        let start = self.pos;
        let mark = lexicon::operator("fnref").expect("fnref punctuation");
        let name_start = start + mark.len();
        if name_start < self.len() && self.byte(name_start).is_ascii_lowercase() {
            let mut end = name_start;
            while end < self.len()
                && (self.byte(end).is_ascii_lowercase() || self.byte(end).is_ascii_digit())
            {
                end += 1;
            }
            let name = self.text[name_start..end].to_string();
            self.pos = end;
            Some(self.token(TokenKind::FnRef(name), start, end))
        } else {
            self.invalid_token();
            None
        }
    }

    fn field(&mut self) -> Option<Token> {
        let start = self.pos;
        let mark = lexicon::operator("field").expect("field punctuation");
        let name_start = start + mark.len();
        if name_start < self.len() && self.byte(name_start).is_ascii_lowercase() {
            let mut end = name_start;
            while end < self.len()
                && (self.byte(end).is_ascii_lowercase() || self.byte(end).is_ascii_digit())
            {
                end += 1;
            }
            let name = self.text[name_start..end].to_string();
            self.pos = end;
            Some(self.token(TokenKind::Field(name), start, end))
        } else {
            self.invalid_token();
            None
        }
    }

    fn invalid_char(&mut self) {
        let start = self.pos;
        let ch = self.text[start..]
            .chars()
            .next()
            .expect("invalid character is present");
        self.push_error("L001", start + 1, "invalid character");
        self.pos += ch.len_utf8();
    }

    fn invalid_token(&mut self) {
        let start = self.pos;
        let end = if is_word_byte(self.byte(start)) {
            self.consume_word_tail(start)
        } else {
            start + 1
        };
        self.push_error("L002", start + 1, "invalid token");
        self.pos = end;
    }

    fn consume_word_tail(&self, mut index: usize) -> usize {
        while index < self.len() && is_word_byte(self.byte(index)) {
            index += 1;
        }
        index
    }
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_ok(source: &str) -> Vec<Line> {
        let (lines, diagnostics) = lex("t.aph", source);
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        lines
    }

    fn tokens(source: &str) -> Vec<Token> {
        lex_ok(source)
            .into_iter()
            .flat_map(|line| line.tokens)
            .collect()
    }

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokens(source).into_iter().map(|token| token.kind).collect()
    }

    fn diagnostics(source: &str) -> Vec<Diagnostic> {
        lex("t.aph", source).1
    }

    fn ident(name: &str) -> TokenKind {
        TokenKind::Ident(name.to_string())
    }

    fn kw(concept: &str) -> TokenKind {
        TokenKind::Keyword(
            lexicon::keyword(concept)
                .expect("known keyword concept")
                .to_string(),
        )
    }

    #[test]
    fn empty_source_is_silent() {
        assert!(lex_ok("").is_empty());
        assert!(lex_ok("\n \n").is_empty());
    }

    #[test]
    fn line_kinds_and_empty_lines() {
        let lines = lex_ok("fn f -> i64\n\n   \n 1   \n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].lineno, 1);
        assert_eq!(lines[0].kind, LineKind::TopLevel);
        assert_eq!(lines[1].lineno, 4);
        assert_eq!(lines[1].kind, LineKind::Body);
        assert_eq!(lines[1].tokens.len(), 1);
        assert_eq!(lines[1].tokens[0].kind, TokenKind::Int("1".to_string()));
        assert_eq!(lines[1].tokens[0].text, "1");
    }

    #[test]
    fn missing_final_lf_is_accepted() {
        let with_lf = lex_ok("fn f -> i64\n 1\n");
        let without_lf = lex_ok("fn f -> i64\n 1");
        assert_eq!(with_lf, without_lf);
    }

    #[test]
    fn keywords_come_from_the_lexicon() {
        for spelling in lexicon::keyword_names() {
            let toks = tokens(spelling);
            assert_eq!(toks.len(), 1, "keyword {spelling}");
            assert_eq!(
                toks[0].kind,
                TokenKind::Keyword(spelling.to_string()),
                "keyword {spelling}"
            );
            assert_eq!(toks[0].text, spelling);
        }
    }

    #[test]
    fn type_names_and_identifiers_lex_as_idents() {
        for spelling in lexicon::type_names() {
            assert_eq!(kinds(spelling), vec![ident(spelling)], "type {spelling}");
        }
        assert_eq!(
            kinds("x x1 abc0"),
            vec![ident("x"), ident("x1"), ident("abc0")]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            kinds("0 -5 42 3.14 -2.5 0.0"),
            vec![
                TokenKind::Int("0".to_string()),
                TokenKind::Int("-5".to_string()),
                TokenKind::Int("42".to_string()),
                TokenKind::Float("3.14".to_string()),
                TokenKind::Float("-2.5".to_string()),
                TokenKind::Float("0.0".to_string()),
            ]
        );
    }

    #[test]
    fn minus_is_an_operator_unless_it_begins_a_literal() {
        assert_eq!(kinds("-5"), vec![TokenKind::Int("-5".to_string())]);
        assert_eq!(
            kinds("- 5"),
            vec![
                TokenKind::Op("-".to_string()),
                TokenKind::Int("5".to_string()),
            ]
        );
        assert_eq!(
            kinds("a - b"),
            vec![ident("a"), TokenKind::Op("-".to_string()), ident("b"),]
        );
    }

    #[test]
    fn strings_store_decoded_value_and_raw_text() {
        let toks = tokens(r#""hello world" "a\nb\t\"\\" "café""#);
        assert_eq!(toks[0].kind, TokenKind::Str("hello world".to_string()));
        assert_eq!(toks[0].text, r#""hello world""#);
        assert_eq!(toks[1].kind, TokenKind::Str("a\nb\t\"\\".to_string()));
        assert_eq!(toks[1].text, r#""a\nb\t\"\\""#);
        assert_eq!(toks[2].kind, TokenKind::Str("café".to_string()));
        assert_eq!(toks[2].text, r#""café""#);
    }

    #[test]
    fn fnref_and_field() {
        let toks = tokens("@add .x");
        assert_eq!(toks[0].kind, TokenKind::FnRef("add".to_string()));
        assert_eq!(toks[0].text, "@add");
        assert_eq!(toks[1].kind, TokenKind::Field("x".to_string()));
        assert_eq!(toks[1].text, ".x");
    }

    #[test]
    fn params_include_the_colon_and_type_follows() {
        let toks = tokens("x:i64 y:list i64");
        assert_eq!(toks[0].kind, TokenKind::Param("x".to_string()));
        assert_eq!(toks[0].text, "x:");
        assert_eq!(toks[0].col, 1);
        assert_eq!(toks[1].kind, ident("i64"));
        assert_eq!(toks[1].col, 3);
        assert_eq!(toks[2].kind, TokenKind::Param("y".to_string()));
        assert_eq!(toks[3].kind, ident("list"));
        assert_eq!(toks[4].kind, ident("i64"));
    }

    #[test]
    fn brackets_attach_to_elements() {
        assert_eq!(
            kinds("[1 2 3]"),
            vec![
                TokenKind::LBracket,
                TokenKind::Int("1".to_string()),
                TokenKind::Int("2".to_string()),
                TokenKind::Int("3".to_string()),
                TokenKind::RBracket,
            ]
        );
        assert_eq!(
            kinds("[[1"),
            vec![
                TokenKind::LBracket,
                TokenKind::LBracket,
                TokenKind::Int("1".to_string()),
            ]
        );
        assert_eq!(
            kinds("3]"),
            vec![TokenKind::Int("3".to_string()), TokenKind::RBracket]
        );
        assert_eq!(kinds("]]"), vec![TokenKind::RBracket, TokenKind::RBracket]);
    }

    #[test]
    fn eq_and_arrow() {
        assert_eq!(
            kinds("ex f 1 = 1"),
            vec![
                kw("example"),
                ident("f"),
                TokenKind::Int("1".to_string()),
                TokenKind::Eq,
                TokenKind::Int("1".to_string()),
            ]
        );
        assert_eq!(
            kinds("fn f -> i64"),
            vec![kw("function"), ident("f"), TokenKind::Arrow, ident("i64"),]
        );
    }

    #[test]
    fn bar_and_match() {
        assert_eq!(
            kinds("type t | a x:i64 | b"),
            vec![
                kw("record"),
                ident("t"),
                TokenKind::Bar,
                ident("a"),
                TokenKind::Param("x".to_string()),
                ident("i64"),
                TokenKind::Bar,
                ident("b"),
            ]
        );
        assert_eq!(kinds("match s"), vec![kw("match"), ident("s")]);
    }

    #[test]
    fn all_binary_operators_come_from_the_lexicon() {
        for spelling in lexicon::binary_operator_names() {
            assert_eq!(
                kinds(spelling),
                vec![TokenKind::Op(spelling.to_string())],
                "operator {spelling}"
            );
        }
    }

    #[test]
    fn capnames_only_after_use() {
        let toks = tokens(
            "use fs.read \"data\"\nuse fs.write \"out\"\nuse net.host \"h\"\nuse env\nuse time\n",
        );
        let capnames: Vec<&str> = toks
            .iter()
            .filter_map(|token| match &token.kind {
                TokenKind::CapName(name) => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(capnames, ["fs.read", "fs.write", "net.host", "env", "time"]);
        assert_eq!(kinds("env time"), vec![ident("env"), ident("time")]);
        assert_eq!(kinds(".read"), vec![TokenKind::Field("read".to_string())]);
    }

    #[test]
    fn whitespace_is_tolerated() {
        let lines = lex_ok("fn   f  ->  i64\n  1  \n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].kind, LineKind::Body);
        assert_eq!(
            lines[0]
                .tokens
                .iter()
                .map(|t| t.kind.clone())
                .collect::<Vec<_>>(),
            vec![kw("function"), ident("f"), TokenKind::Arrow, ident("i64"),]
        );
    }

    #[test]
    fn columns_are_one_based_byte_columns() {
        let (lines, diagnostics) = lex("t.aph", "é 1");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].col, 1);
        assert_eq!(lines[0].tokens[0].kind, TokenKind::Int("1".to_string()));
        assert_eq!(lines[0].tokens[0].col, 4);

        let toks = tokens("\"é\" x");
        assert_eq!(toks[0].col, 1);
        assert_eq!(toks[1].col, 6);
    }

    #[test]
    fn l001_invalid_characters() {
        for (source, col) in [("fn\tf", 3), ("a\r", 2), ("é", 1), ("\u{1}", 1)] {
            let found = diagnostics(source);
            assert_eq!(found.len(), 1, "source {source:?}");
            assert_eq!(found[0].code, "L001", "source {source:?}");
            assert_eq!(found[0].msg, "invalid character");
            assert_eq!(found[0].col, col, "source {source:?}");
        }
    }

    #[test]
    fn l001_does_not_apply_to_printable_non_ascii_inside_strings() {
        assert_eq!(kinds("\"é\""), vec![TokenKind::Str("é".to_string())]);
    }

    #[test]
    fn l001_raw_control_characters_in_strings() {
        for (source, col) in [
            ("\"a\tb\"", 3),
            ("\"a\rb\"", 3),
            ("\"\u{1}\"", 2),
            ("\"\u{7f}\"", 2),
            ("\"\u{85}\"", 2),
        ] {
            let found = diagnostics(source);
            assert_eq!(found.len(), 1, "source {source:?}");
            assert_eq!(found[0].code, "L001", "source {source:?}");
            assert_eq!(found[0].msg, "invalid character", "source {source:?}");
            assert_eq!(found[0].col, col, "source {source:?}");
        }
    }

    #[test]
    fn l002_invalid_tokens() {
        for source in ["Foo", "x_y", "01", "1.", "@", "!", "_x"] {
            let found = diagnostics(source);
            assert_eq!(found.len(), 1, "source {source:?}");
            assert_eq!(found[0].code, "L002", "source {source:?}");
            assert_eq!(found[0].msg, "invalid token");
        }
        assert_eq!(diagnostics("Foo")[0].col, 1);
        assert_eq!(diagnostics("x_y")[0].col, 2);
    }

    #[test]
    fn l002_bad_strings() {
        let bad_escape = diagnostics(r#""\q""#);
        assert_eq!(bad_escape.len(), 1);
        assert_eq!(bad_escape[0].code, "L002");
        assert_eq!(bad_escape[0].col, 2);

        // A bad escape keeps the raw backslash and character in the value.
        let (lines, _) = lex("t.aph", r#""\q""#);
        assert_eq!(lines[0].tokens[0].kind, TokenKind::Str(r"\q".to_string()));

        let unterminated = diagnostics("\"abc");
        assert_eq!(unterminated.len(), 1);
        assert_eq!(unterminated[0].code, "L002");
        assert_eq!(unterminated[0].col, 1);
    }

    #[test]
    fn diagnostics_are_sorted_by_position() {
        // Unterminated string at col 1, bad escape at col 2; the bad escape is
        // found first but the result is sorted by position.
        let found = diagnostics("\"\\q");
        assert_eq!(found.len(), 2);
        assert_eq!(
            found.iter().map(|d| (d.line, d.col)).collect::<Vec<_>>(),
            [(1, 1), (1, 2)]
        );
    }

    #[test]
    fn collects_all_errors() {
        let found = diagnostics("Foo\n01\né");
        assert_eq!(
            found.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
            ["L002", "L002", "L001"]
        );
        assert_eq!(found.iter().map(|d| d.line).collect::<Vec<_>>(), [1, 2, 3]);
    }

    #[test]
    fn spec_examples_lex_cleanly() {
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
        for example in examples {
            let lines = lex_ok(example);
            assert!(!lines.is_empty());
            assert_eq!(lines[0].kind, LineKind::TopLevel);
        }
    }
}
