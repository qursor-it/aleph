# ADR-0017: Float literal text is preserved verbatim

Status: Proposed
Date: 2026-11-15

## Context
ADR-0003 makes the AST the source of truth and defines one canonical text per
program. Numeric literals raise a question: `1.0` and `1.00` denote the same
`f64` value, but printing the parsed value back can change the source between
parse/print round trips. Rust's default float formatting may also emit exponent
forms (`1e-3`, `1E3`, `inf`, `NaN`) that the v0.1 Aleph grammar and lexer do not
accept, so it cannot provide a safe canonical spelling today.

## Decision
Float literals keep their source token text verbatim in the AST, as
`ExprKind::Float { value: String }`. The printer emits that text unchanged.
Consequently `1.0` and `1.00` are distinct canonical texts in v0.1: both
round-trip byte-for-byte, even though they evaluate to the same value.

Integer literals continue to use their numeric value (`ExprKind::Int { value:
i64 }`); this ADR only covers floats.

Normalization to one spelling per numeric value is deferred. Any future
normalization must define an Aleph-native decimal format and update the lexer
and grammar first; it must not depend on host-language float formatting.

## Consequences
- `print(parse(t)) == t` holds for float literals as written.
- The parser must retain the raw float token text; it must not parse and
  re-format the literal.
- Type checking and interpretation may parse the text into an `f64`; the AST
  keeps the original spelling.
- Equal-value floats with different spellings remain textually different, so
  canonical-form checks (F001) do not rewrite them.
- A later ADR may establish numeric normalization and become the new rule.
