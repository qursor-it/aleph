# Diagnostics

Stable diagnostic codes (SPEC §9, ADR-0011, ADR-0015). Codes are an API: never
reuse a code with a different meaning. The Rust table in `compiler/src/diag.rs`
must match this table exactly.

| Code | Meaning |
|---|---|
| L001 | invalid character |
| L002 | invalid token |
| P001 | unexpected token |
| P002 | wrong number of arguments (arity) |
| P003 | unknown function |
| F001 | not in canonical form (include `canonical`) |
| T001 | type mismatch |
| T005 | shadowed binding |
| T010 | cannot infer type of ok/err/none |
| E001 | missing examples |
| E002 | example failed (expected/found) |
| C001 | missing capability |
| C002 | path not covered by capability |
| R001 | integer overflow |
| R002 | division by zero |
| R003 | capability violation at run time |
| X001 | command not implemented |
| X002 | cannot read file |
