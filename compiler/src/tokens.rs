//! Token counters for `aleph tokens` (ADR-0031).
//!
//! No BPE vocabulary ships with the repository, so every counter is a local,
//! deterministic approximation. A real tokenizer is one more table entry.

use crate::{lexer, llms};

/// A named token counter. `count` returns `None` when the text cannot be
/// tokenized by this counter (the `aleph` counter on a lexical error).
pub struct Tokenizer {
    pub name: &'static str,
    pub method: &'static str,
    pub count: fn(&str) -> Option<usize>,
}

/// All counters; the first is the default.
pub const TOKENIZERS: &[Tokenizer] = &[
    Tokenizer {
        name: "chars4",
        method: "ceil(unicode chars / 4), the usual LLM estimate",
        count: |text| Some(llms::estimated_tokens(text)),
    },
    Tokenizer {
        name: "bytes",
        method: "utf-8 bytes",
        count: |text| Some(text.len()),
    },
    Tokenizer {
        name: "chars",
        method: "unicode scalar values",
        count: |text| Some(text.chars().count()),
    },
    Tokenizer {
        name: "words",
        method: "whitespace-separated words",
        count: |text| Some(text.split_whitespace().count()),
    },
    Tokenizer {
        name: "aleph",
        method: "aleph lexer tokens (SPEC §2)",
        count: aleph_tokens,
    },
];

fn aleph_tokens(text: &str) -> Option<usize> {
    let (lines, diagnostics) = lexer::lex("", text);
    diagnostics
        .is_empty()
        .then(|| lines.iter().map(|line| line.tokens.len()).sum())
}

/// The counters selected by `--tokenizer` (`None` = default, `all` = every
/// counter), or `None` for an unknown name.
pub fn select(name: Option<&str>) -> Option<Vec<&'static Tokenizer>> {
    match name {
        None => Some(vec![&TOKENIZERS[0]]),
        Some("all") => Some(TOKENIZERS.iter().collect()),
        Some(name) => TOKENIZERS
            .iter()
            .find(|tokenizer| tokenizer.name == name)
            .map(|tokenizer| vec![tokenizer]),
    }
}

/// Comma-separated counter names, for usage errors.
pub fn names() -> String {
    TOKENIZERS
        .iter()
        .map(|tokenizer| tokenizer.name)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(name: &str, text: &str) -> Option<usize> {
        (select(Some(name)).expect("known")[0].count)(text)
    }

    #[test]
    fn counters_measure_what_they_say() {
        let text = "fn main -> unit\n print \"hé\"\n";
        assert_eq!(count("bytes", text), Some(text.len()));
        assert_eq!(count("chars", text), Some(text.len() - 1));
        assert_eq!(count("chars4", text), Some((text.len() - 1).div_ceil(4)));
        assert_eq!(count("words", text), Some(6));
        // fn main -> unit / print "hé"
        assert_eq!(count("aleph", text), Some(6));
        assert_eq!(count("aleph", "fn main -> unit\n print $\n"), None);
    }

    #[test]
    fn selection() {
        assert_eq!(select(None).unwrap()[0].name, "chars4");
        assert_eq!(select(Some("all")).unwrap().len(), TOKENIZERS.len());
        assert!(select(Some("gpt4")).is_none());
        assert_eq!(names(), "chars4, bytes, chars, words, aleph");
    }
}
