# ADR-0041: `[]` is an empty list whose element type comes from the context

Status: Proposed
Date: 2026-11-20

## Context
An empty list is written `empty T` (SPEC §5); `[]` is P001 "empty list".
Models write `[]` by habit from every other language: in the 18 Nov pilot
(lite spec) 3 of the 8 compile errors left after formatting were this P001
(T-054 replay). `ok`, `err` and `none`-free results already take their type
from the expected type (bidirectional checking, T010 when there is none), so
the checker has the mechanism.

Options (score 1-10):
- **`[]` is an expression of type `list A` where A comes from the expected
  type, like `ok`/`err`; T010 when nothing fixes it; `empty T` stays.** One
  habit stops being an error; no new rule for the model to learn, only one
  fewer. Canonical text of the new node is `[]`. **8**
- **`aleph fmt` rewrites `[]` to `empty T`.** One spelling, but the printer
  would need types, which it does not have (the AST is untyped). **4**
- **Better P001 message with a fix to `empty T`.** Still a failed round; the
  fix needs the type, which the parser does not know. **3**

## Decision
- Grammar: `[]` (the two tokens `[` and `]` with nothing between) is an
  expression node `EmptyList`. `[e..]` with n >= 1 is unchanged.
- Typing: `[]` checks against an expected type `list T` and has that type.
  With no expected type, or one where a list's element type is unresolved
  after checking the expression it appears in, it is T010 (message "cannot
  infer type of ok/err/none/[]", shared with `ok`/`err`/`none`) with
  `expected` naming `empty T`. As an argument of a generic
  builtin it is typed like `ok`/`err` arguments (after the call's expected
  type, ADR-0037 checking order). Against a non-list expected type it is
  T001.
- `empty T` keeps its meaning and canonical text; the printer prints `[]` as
  `[]` (different AST node, so ADR-0003 still holds).
- Both engines produce the same empty list value; `show []` prints `[]` as
  today for an empty list.
- llms.txt: `[e..]` n>=1 becomes `[e..]` (`[]` empty, type from context;
  else `empty T`).

## Amendments
- 2026-11-20 (review): an inferred type is rejected when it contains an
  unresolved variable anywhere (`list V` from `[]` inside a lambda), not
  only at the top level; otherwise `check` passed and `build` failed with
  X006. Typing follows the checking order, like `ok`/`err`: `[[1] []]` and
  `if c xs []` check, `[[] [1]]` and `if c [] xs` in a `let` are T010, and a
  later use of a `let` does not type an earlier `[]`.

## Consequences
- The most common habit error in the pilot is no longer an error when the
  type is known (function result, typed parameter, `push [] x`, a branch of
  `if`, the default of `else`, a `map` lambda result).
- `let xs []` alone is T010 (no expected type): the model writes `empty T`
  there, as today.
