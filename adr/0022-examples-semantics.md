# ADR-0022: Inline example semantics

Status: Proposed
Date: 2026-11-15

## Context
ADR-0010 makes inline examples mandatory and executes them during `aleph check`
and `aleph test`. SPEC §3.3 and §12 say which functions need examples, but leave
open how "effectful" is computed, how examples are evaluated, and how a failing
example is reported. The examples runner (T-014) needs one deterministic answer
shared with later tools.

## Decision
- **Effectfulness is transitive and body-only.** A function is effectful when
  its statements and result reach an effectful builtin (`print`, `eprint`,
  `read`, `write`) directly, through a call to a user function, or through a
  `@fnref` argument (for example `map @f xs`). Examples are test code and do
  not make their function effectful. The set is the least fixpoint over the
  body call graph and is computed deterministically (sorted maps and sets).
  `main` never needs an example; effectful functions are exempt from E001.
- **E001.** Every other function with no `ex` line reports `E001` "missing
  examples" at the function header's node, line and column.
- **Evaluation is hermetic.** Each example's left and right expressions are
  evaluated with the interpreter in an empty environment, with empty program
  arguments, with stdout/stderr sent to discarding sinks, and with filesystem
  builtins denied: any `read` or `write` call traps `R003` without resolving a
  path or touching the disk (`Interp::with_deny_fs`). Other capability checks
  follow the same runtime rules. A `try` that returns early at example top
  level yields its `err e` value.
- **Comparison.** Values are compared with `interp::values_equal` (structural;
  floats use IEEE `==`, so `NaN != NaN`). A mismatch reports `E002` at the
  example node, with `expected` = the right-hand value and `found` = the
  left-hand value.
- **Traps.** A trap on the left reports `E002` with `found = "trap R00x"` and
  `expected` = the right-hand value. A trap on the right reports
  `expected = "trap R00x"` and `found` = the left-hand value. When both sides
  trap, `found` is the left trap and `expected` is the right trap.
- **Value notation.** `interp::literal` renders values deterministically in
  Aleph value notation: `i64`/`f64` as `show`, `bool`, quoted and escaped
  `str`, `nil`, `[a b]` / `[]`, `some X` / `none`, `ok X` / `err X`, and
  records in constructor form `NAME f1 f2`. Nested values use the same prefix
  syntax without parentheses. This is human-readable diagnostic notation, not
  guaranteed re-parseable source: `none` omits its element type, `[]` is not a
  source literal, and non-finite floats (`inf`, `NaN`) are not float literals.
- **Commands.** `aleph check` runs `E001` and `E002` after F001, type and
  capability checks, and runs every example even when earlier examples fail.
  Diagnostics are sorted by `(line, col)`. `aleph test` performs the same
  lex/parse/type/capability pipeline without F001, runs examples only (no E001)
  and reports `E002` failures as JSON lines; a clean run has no output and
  exits 0. `X001` is no longer emitted by `aleph check`.

## Consequences
- Every non-effectful function is self-testing, and examples can call effectful
  helpers without leaking their output into the compiler's stdout/stderr or
  mutating the filesystem.
- Example failures are structured and diff-friendly: `expected` and `found` are
  values in Aleph's diagnostic value notation.
