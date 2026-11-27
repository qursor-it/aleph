# ADR-0036: Builtins `abs min max starts ends uniq map2`

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-18

## Context
The token pilot of 18 Nov (`experiments/pilot/tokens/`) counted 77 verified
programs in Aleph and Python. Aleph bodies are shorter than Python's (median
ratio 0.74), but 39% of Aleph's tokens are function signatures: each missing
builtin becomes a named, fully typed helper. The most frequent ones in the
corpus were absolute value, minimum and maximum, string prefix and suffix
tests, a sorted set of a list, and pairwise mapping of two lists (`zip`).
Re-counting the HumanEval part of the corpus with these builtins moves the
median Aleph/Python ratio from 1.23 to 1.08 (`whatif.py`).

## Decision
Seven pure builtins (SPEC §5.1):

| Name | Signature |
|---|---|
| `abs x` | T -> T (T `i64` or `f64`); `i64` minimum traps R001 |
| `min a b` / `max a b` | T, T -> T (T `i64` or `f64`); `b` when `b < a` (`b > a`), else `a`, so with NaN the first argument is kept |
| `starts s p` / `ends s p` | str, str -> bool (bytes) |
| `uniq xs` | list T -> list T (T primitive): ascending, duplicates removed, same order as `sort` |
| `map2 @f xs ys` | (A, B -> T), list A, list B -> list T; length of the shorter list |

A new bound `numeric` (`i64` or `f64`) expresses the restriction of `abs`,
`min` and `max`; a wrong type is T004 with `expected: numeric`.

## Consequences
- Seven more reserved names: programs that used them as function, parameter
  or local names stop compiling (P001/T005). Acceptable before v1.
- `llms.txt` grows by eight lines (about 60 tokens).
- No new syntax, no change to codegen: the C backend calls `aleph_b_NAME` as
  for every builtin; `map2` uses the existing callback path.
- ADR-0035 is left for T-044 (`ord`/`chr`).
