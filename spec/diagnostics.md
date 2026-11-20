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
| T002 | unknown type |
| T003 | invalid field access |
| T004 | type constraint not satisfied |
| T005 | shadowed binding |
| T006 | function reference not allowed here |
| T007 | try outside result function |
| T008 | invalid main signature |
| T010 | cannot infer type of ok/err/none |
| E001 | missing examples |
| E002 | example failed (expected/found) |
| C001 | missing capability |
| C002 | path not covered by capability |
| C003 | invalid capability |
| R001 | integer overflow |
| R002 | division by zero |
| R003 | capability violation at run time |
| R004 | stack overflow |
| R005 | allocation limit exceeded |
| X001 | command not implemented |
| X002 | cannot read file |
| X003 | no main function |
| X004 | c compiler failed |
| X005 | reserved (no longer emitted) |
| X006 | internal compiler error |
| X007 | invalid edits json |
| X008 | unknown node id |
| X009 | edit not allowed for node kind |
| X010 | edit does not parse |
| X011 | conflicting edits |

F001 has severity `warning` and no `fix` in default `aleph check` (checking goes on); under `--strict` it is an error with a whole-file `fix` and stops checking (ADR-0040).
