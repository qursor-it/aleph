# ADR-0019: Type-checker diagnostic codes and type spellings

Status: Proposed
Date: 2026-11-15

## Context
SPEC §9 lists only `T001` (type mismatch), `T005` (shadowed binding) and
`T010` (cannot infer the open side of `ok`/`err`/`none`). The T-010 checker
also needs to report unknown named types, invalid field access, violated
builtin bounds, misplaced function references, `try` outside a result
function, and an invalid `main` signature. Diagnostics also need a canonical
spelling for the `expected`/`found` fields, including function types and
inference variables that never occur in source types.

## Decision
Add the following stable type codes (also in `spec/diagnostics.md`):

| Code | Meaning |
|---|---|
| T002 | unknown type |
| T003 | invalid field access |
| T004 | type constraint not satisfied |
| T006 | function reference not allowed here |
| T007 | try outside result function |
| T008 | invalid main signature |

Diagnostics use the canonical type spelling from SPEC §4: `i64`, `f64`,
`bool`, `str`, `unit`, `list T`, `opt T`, `res T E`, and record names. Two
diagnostics-only forms complete it:

- a function type prints `(A B) -> R`, with one space between parameters;
  `() -> R` has an empty parameter list;
- an inference variable that is still unresolved prints `_`.

This spelling is exposed as `impl Display for Type` so later phases such as
`aleph explain` (T-016) reuse it. `Error` (the recovery type behind T002 and
friends) is internal and prints `error` if it ever reaches a diagnostic; in
practice it unifies silently and is not reported.

`T004` is used both for builtin bounds (`sort`'s `primitive`, `show`'s
`showable`) and for `==` / `!=` on non-primitive operands. When both operands
of an arithmetic, comparison or equality operator are still unresolved
inference variables, the constraint (`numeric` or `primitive`) is deferred and
checked after the function body instead of reporting a mismatch against `_`.
Examples in the type checker are checked like ordinary expressions and are not
restricted to primitive arguments or results; T-014 will compare example
outcomes structurally once the interpreter exists.

`@f` is only a value where a function type is expected, and function
references to effectful builtins (`print`, `eprint`, `read`, `write`) are
`T006`. Capability checking stays on `Call` nodes only, so an effectful
builtin cannot escape to a higher-order call such as `map @read xs`.

Record types may refer to themselves, directly or through `opt`/`list`/`res`:
record names are collected before fields are lowered. For example
`type node v:i64 next:opt node` is valid and constructible (`node 1 none node`)
and produces no diagnostic.

## Consequences
- The code table stays stable and append-only; existing meanings do not
  change.
- `expected`/`found` are deterministic and machine-readable for all type
  diagnostics that carry them.
- Function values remain second-class: `@f` is only well typed where an
  expected function type exists (arguments of `map`, `filter`, `fold`). A
  binding such as `let g @f` is T006 rather than an inferred function type.
- Any future higher-order surface syntax needs a new ADR that supersedes this
  one.
