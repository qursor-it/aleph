# ADR-0053: Builtin `nth` (list index that traps out of range)

Status: Proposed
Date: 2026-11-22

## Context
`at xs i` returns `opt T`, so every index a program knows to be valid is
written `else at xs i D` with a dummy default D the program never means:
320 times in the dev probe answers (experiments/devprobe, 14 runs), the
largest single pattern where model-written Aleph is longer than the same
model's Python (`m[i]`; T-081: model Aleph is 0.98-1.11x its Python). A
dummy default also hides a wrong index as a wrong value instead of failing.
Aleph already traps on overflow (R001) and division by zero (R002).

Options (score 1-10):
- **`nth xs i : list T, i64 -> T`, trap R006 "index out of range"** when
  i < 0 or i >= len. Name from OCaml `List.nth` / Lisp `nth`, which fail
  out of range. `at` stays for "maybe absent". Saves `else` and the default
  (about 8 chars, 2-3 tokens per use). **7**
- **Make `at` trap and return T.** Breaks every program and example. **2**
- **Keep as is.** **3**

## Decision
- `nth xs i : list T, i64 -> T`; R006 "index out of range" (exit 70, JSON
  on stderr, as other traps) when out of range; in an `ex` the example
  fails with found = the trap. Pure otherwise; both engines identical.
- `nth` is reserved (T005); `nth` usable as `@nth` like other pure builtins.
- llms.txt: table line `nth list T,i64->T (R006 out of range)`; the first
  example's `ex else at [10 20] 1 0 = 20` becomes `ex nth [10 20] 1 = 20`.

## Consequences
- `nth m i` instead of `else at m i empty i64`; wrong indexes fail loudly.
- One more trap code (R001-R006) in SPEC §9 / diagnostics.md.
