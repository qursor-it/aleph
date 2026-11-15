use serde::Serialize;

/// A diagnostic as specified by SPEC §9 and ADR-0011.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub msg: String,
}

impl Diagnostic {
    /// X001: the requested command is not implemented yet (ADR-0015).
    pub fn not_implemented(file: &str) -> Self {
        Self {
            code: "X001".to_string(),
            severity: "error".to_string(),
            file: file.to_string(),
            line: 0,
            col: 0,
            msg: "not implemented".to_string(),
        }
    }
}
