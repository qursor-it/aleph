# ADR-0051: `len` also takes a `str` (byte count, like `count`)

Status: Proposed
Date: 2026-11-22

## Context
`len` was `list T -> i64` and `count` `str -> i64` (bytes). Writing
`len STR` is the most frequent T001 seen in the dev probe across models
(experiments/devprobe): DeepSeek run9 `wrap`; Claude Haiku run13 `wrap` and
`fromroman` (5 of its 12 errors on set 4). ADR-0050 made that T001 name
`count`, which fixes it in one repair turn, but the turn and the error still
cost tokens, and every model arrives with `len` from Python. `cat` already
takes `str` or `list` (ADR-0045), so a builtin over both is not new.

Options (score 1-10):
- **`len` takes `str` or `list U`** (bound `StrOrList`, as `cat`); on a str
  it is the byte count, same as `count`. No new name, error class gone,
  llms.txt line changes from `len list T->i64` to `len T->i64 (T str or
  list)`. **8**
- **Keep the ADR-0050 hint only.** Costs a check/repair turn each time. **4**
- **Remove `count`, keep only `len`.** Breaks existing programs; can be done
  later if `count` goes unused. **3**

## Decision
- `len x : T -> i64` with T `str` or `list U`: a list's element count, a
  str's byte count (the same value as `count`, chars are `len split s ""`).
- Both engines identical: the interpreter and `aleph_b_len` dispatch on the
  value's tag. SPEC §5.1 row updated; llms.txt line and semantics note.
- ADR-0050's `len` half is superseded (no error to hint); its `count LIST`
  hint stays.

## Consequences
- `len s` checks and runs; `type_len_count` golden keeps only the `count`
  case; `run_len_str` covers both engines.
- Byte length, not characters, as `count`: `len "héllo"` is 6.
