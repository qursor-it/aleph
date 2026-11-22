# ADR-0050: T001 names `count`/`len` for the other length

Status: Proposed (its `len` half superseded by ADR-0051)
Date: 2026-11-22

## Context
Aleph has two lengths: `len list T->i64` and `count str->i64`. Models write
Python's `len` on a string (dev probe run9 `wrap`: `len cur` with `cur:str`,
the run's only error). The T001 says `expected list _ found str`, which does
not name `count`. The opposite slip, `count xs` on a list, got a `fix`
`show xs`: it type checks but `count show xs` is not a length.

Options (score 1-10):
- **Name the other builtin in the T001 message** (`type mismatch: str length
  is \`count\``, `type mismatch: list length is \`len\``) and drop the
  misleading `show` fix in that case. Free in llms.txt; exact. **8**
- **`len` accepts str too** (bound str-or-list, like `cat`). Removes the
  error, but two names for the same str length, and `len` on str would have
  to choose bytes or characters. **5**
- **Nothing.** **2**

## Decision
When the first argument of `len` is found `str`, or that of `count` is
found `list ..`, the T001 on it uses the message above and carries no
`fix`. Everything else in T001 is unchanged.

## Consequences
- Two message strings; deterministic; golden `type_len_count`.
