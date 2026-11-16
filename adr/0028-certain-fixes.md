# ADR-0028: Certain `fix` suggestions for common diagnostics

Status: Proposed
Date: 2026-11-15

## Context
SPEC §9 and ADR-0011 allow a `fix` object `{"op","node","with"}` in a
diagnostic, but only when the correction is certain. T-023 identifies ten
mechanical corrections that an agent can apply without guessing. A wrong fix
is worse than no fix, so every rule has an explicit certainty guard.

## Decision
Emit `fix` only for the rules below. `n` is the diagnostic's expression node
id, `E` is that expression's canonical printed text (`printer::print_expr`),
and `with` is canonical source text.

1. `F001` -> `{"op":"replace","node":"n0","with":CANONICAL}`, where `CANONICAL`
   is `printer::print_file` for the parsed file. Node `n0` is the file root.
2. `T001` expected `res T U`, found `T`, with `T != U` -> `replace n` with
   `ok E`.
3. `T001` expected `res T U`, found `U`, with `T != U` -> `replace n` with
   `err E`.
4. `T001` expected `opt T`, found `T` -> `replace n` with `some E`.
5. `T001` expected `bool`, found `opt _` -> `replace n` with `has E`.
6. `T001` expected `bool`, found `res _ _` -> `replace n` with `isok E`.
7. `T001` expected `T`, found `res T U`, where `T` is not `bool` and the
   enclosing function returns `res V U` (same error type `U`) -> `replace n`
   with `try E`.
8. `T001` expected `str`, found a showable non-`str` type (a primitive or a
   list of primitives) -> `replace n` with `show E`.
9. `C001` for `read`/`write` whose path argument is a string literal that is
   non-empty, relative (no leading `/`, SPEC §8), has no `..` segment and
   does not normalize to the whole working directory (`.`, `./`: granting
   all of it is never a certain fix) -> `{"op":"insert","node":"n0","with":
   "use fs.read \"<literal>\""}` (or `fs.write`). The literal is escaped with
   `ast::escape_str`.
10. `C002` with the same literal conditions -> `insert` at `n0` the `use`
    line for the literal path itself, e.g. `use fs.read "other/x"`.
    When several C001/C002 diagnostics in a file would insert the same `use`
    line, only the first (in line/col order) carries the fix.

### Certainty conditions
- The expected and found types must be fully resolved: no `Var` and no
  `Error` anywhere in either type. An unresolved type variable could be bound
  differently by a later check.
- Rules 2 and 3 require `T != U`; when the result's two sides are equal the
  intended wrapper is ambiguous, so no fix is emitted.
- Rule 7 additionally requires the enclosing return type to be a result with
  exactly the same error type; `try` otherwise fails at that position.
- Rule 8 excludes `str` (already correct) and non-showable types (records,
  nested lists, options, results, functions).
- Capability fixes require an actual string-literal path with a segment-safe,
  valid prefix: non-empty, relative and no `..`. Computed, empty, absolute
  paths and paths with `..` get no fix.
- No fix is attached to `P002`, `T007`, or any diagnostic not listed here.
  These cases need a structural choice that the checker cannot make for the
  author.

### `insert` at `n0`
`n0` is the file root. Inserting a top-level item is textual: add the `use`
line as a top-level item and then apply canonical order. Canonical order sorts
`use` lines by full canonical line text (SPEC §3), so the item's final
position is not necessarily first in the file. `aleph fmt` performs the
ordering; the golden tests apply an insert by prepending the line and
canonicalizing.

## Consequences
- Diagnostics for the listed patterns gain a `fix` field; existing `.diag`
  goldens with those codes change.
- `tests/aph/fix_*.aph` cover one fix each. `compiler/tests/fixes.rs` applies
  every golden fix textually, canonicalizes, and re-checks that the failing
  code is gone.
- Adding a rule requires the same treatment: a certainty argument, unit
  tests including negative cases, and an apply-and-recheck golden.
