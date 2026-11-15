//! Structured diagnostics, per SPEC §9, ADR-0011 and ADR-0015.

use std::io::{self, Write};

use serde::Serialize;

/// A diagnostic as specified by SPEC §9 and ADR-0011.
///
/// `line` and `col` are 1-based for source locations. `line` 0 and `col` 0
/// mean "no location" (ADR-0015).
///
/// Field order here is the field order in the JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub found: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

/// A certain fix for a diagnostic (SPEC §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fix {
    pub op: String,
    pub node: String,
    #[serde(rename = "with")]
    pub with: String,
}

impl Diagnostic {
    /// Build an error diagnostic. `severity` is always `error`.
    pub fn error(
        code: impl Into<String>,
        file: impl Into<String>,
        line: u32,
        col: u32,
        msg: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: "error".to_string(),
            file: file.into(),
            line,
            col,
            node: None,
            msg: msg.into(),
            expected: None,
            found: None,
            fix: None,
            canonical: None,
        }
    }

    /// X001: the requested command is not implemented yet (ADR-0015).
    pub fn not_implemented(file: &str) -> Self {
        Self::error("X001", file, 0, 0, "not implemented")
    }

    pub fn with_node(mut self, node: impl Into<String>) -> Self {
        self.node = Some(node.into());
        self
    }

    pub fn with_expected_found(
        mut self,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        self.expected = Some(expected.into());
        self.found = Some(found.into());
        self
    }

    pub fn with_fix(
        mut self,
        op: impl Into<String>,
        node: impl Into<String>,
        with: impl Into<String>,
    ) -> Self {
        self.fix = Some(Fix {
            op: op.into(),
            node: node.into(),
            with: with.into(),
        });
        self
    }

    pub fn with_canonical(mut self, canonical: impl Into<String>) -> Self {
        self.canonical = Some(canonical.into());
        self
    }

    /// Serialize this diagnostic as one JSON object (no trailing newline).
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).expect("diagnostic is always serializable")
    }

    /// Write diagnostics to `writer`, one JSON object per line.
    pub fn emit_all<W: Write>(diagnostics: &[Diagnostic], writer: &mut W) -> io::Result<()> {
        for diagnostic in diagnostics {
            writeln!(writer, "{}", diagnostic.to_json_line())?;
        }
        Ok(())
    }
}

/// Stable diagnostic codes: `(code, meaning)`. Mirrors `spec/diagnostics.md`.
pub const CODES: &[(&str, &str)] = &[
    ("L001", "invalid character"),
    ("L002", "invalid token"),
    ("P001", "unexpected token"),
    ("P002", "wrong number of arguments (arity)"),
    ("P003", "unknown function"),
    ("F001", "not in canonical form (include `canonical`)"),
    ("T001", "type mismatch"),
    ("T005", "shadowed binding"),
    ("T010", "cannot infer type of ok/err/none"),
    ("E001", "missing examples"),
    ("E002", "example failed (expected/found)"),
    ("C001", "missing capability"),
    ("C002", "path not covered by capability"),
    ("R001", "integer overflow"),
    ("R002", "division by zero"),
    ("R003", "capability violation at run time"),
    ("X001", "command not implemented"),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_spec_table(text: &str) -> Vec<(String, String)> {
        let mut rows = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with('|') {
                continue;
            }
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            if cells.len() != 2 || cells[0] == "Code" {
                continue;
            }
            if cells[0].chars().all(|c| c == '-' || c == ':') {
                continue;
            }
            rows.push((cells[0].to_string(), cells[1].to_string()));
        }
        rows
    }

    #[test]
    fn required_only_serialization_is_exact() {
        let diagnostic = Diagnostic::error("X001", "foo.aph", 0, 0, "not implemented");
        assert_eq!(
            diagnostic.to_json_line(),
            r#"{"code":"X001","severity":"error","file":"foo.aph","line":0,"col":0,"msg":"not implemented"}"#
        );
        assert_eq!(diagnostic, Diagnostic::not_implemented("foo.aph"));
    }

    #[test]
    fn full_serialization_is_exact_and_ordered() {
        let diagnostic = Diagnostic::error("T001", "a.aph", 3, 14, "type mismatch")
            .with_node("n42")
            .with_expected_found("i64", "str")
            .with_fix("replace", "n42", "int n42")
            .with_canonical("fn f x:i64 -> i64");
        assert_eq!(
            diagnostic.to_json_line(),
            r#"{"code":"T001","severity":"error","file":"a.aph","line":3,"col":14,"node":"n42","msg":"type mismatch","expected":"i64","found":"str","fix":{"op":"replace","node":"n42","with":"int n42"},"canonical":"fn f x:i64 -> i64"}"#
        );
    }

    #[test]
    fn msg_and_file_are_json_escaped() {
        let diagnostic = Diagnostic::error("L002", "a\"b\\c.aph", 1, 2, "bad \"quoted\"\nnext\\");
        assert_eq!(
            diagnostic.to_json_line(),
            r#"{"code":"L002","severity":"error","file":"a\"b\\c.aph","line":1,"col":2,"msg":"bad \"quoted\"\nnext\\"}"#
        );
    }

    #[test]
    fn emit_all_writes_one_line_per_diagnostic() {
        let diagnostics = [
            Diagnostic::error("L001", "a.aph", 1, 1, "invalid character"),
            Diagnostic::not_implemented("b.aph"),
        ];
        let mut out = Vec::new();
        Diagnostic::emit_all(&diagnostics, &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            concat!(
                "{\"code\":\"L001\",\"severity\":\"error\",\"file\":\"a.aph\",\"line\":1,\"col\":1,\"msg\":\"invalid character\"}\n",
                "{\"code\":\"X001\",\"severity\":\"error\",\"file\":\"b.aph\",\"line\":0,\"col\":0,\"msg\":\"not implemented\"}\n",
            )
        );
    }

    #[test]
    fn codes_table_matches_spec_diagnostics() {
        let spec = parse_spec_table(include_str!("../../spec/diagnostics.md"));
        let rust: Vec<(String, String)> = CODES
            .iter()
            .map(|(code, meaning)| (code.to_string(), meaning.to_string()))
            .collect();
        assert_eq!(spec, rust);
    }
}
