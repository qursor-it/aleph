# ADR-0042: Builtins `mapr`, `sortby`, `repeat`, `index`

Status: Proposed
Date: 2026-11-20

## Context
The EXP-003 stage A corpus (T-040) counts Aleph at 1.24x Python (median,
chars4, without examples). Per program, Aleph is shorter on most bench
programs (upper 0.51 ... reverse 0.76) and much longer on six: text-top-words
4.10, csv column sum 3.02, histogram 2.77, maxmin 2.56, total 2.21,
files-copy-prefix 1.63. Reading them, four shapes cost the tokens:
- "parse every string, stop at the first error": a `fold` over a `res`
  accumulator plus two helper functions (histogram, maxmin, total, csv);
- ranking by a key: zero-padded complemented string keys (text-top-words);
- `"*" * n`: `join map @star range 0 n ""` plus a helper (histogram);
- position of an element: a recursive helper (csv).
None needs a new rule: each is one builtin with a fixed arity, and builtin
function references (`@int`) already work as higher-order arguments.

Options (score 1-10):
- **Four builtins: `mapr f xs`, `sortby f xs`, `repeat s n`, `index xs x`.**
  Each replaces one helper pattern; names are short and close to what models
  know (`sorted(key=)`, `str * n`, `list.index`). **8**
- **General loops or comprehensions.** Would cover more, but are new syntax
  and rules, which the pilot found to be the real cost. **3**
- **Leave it to examples in llms.txt.** Shows the patterns but keeps every
  program long. **2**

## Decision
- `mapr f xs` : `(A -> res B E), list A -> res (list B) E`. Applies `f` to
  the elements in order; the first `err` is the result and later elements are
  not visited; otherwise `ok` of the list of `ok` values. `f` is `@g`, a
  builtin reference such as `@int`, or `lam x E` (1 binder), as for `map`.
- `sortby f xs` : `(A -> K), list A -> list A`, K one of `i64 f64 str`.
  Stable sort of `xs` by `f x` ascending, same key order as `sort` (f64 NaN
  rule of ADR-0024). `f` is called once per element.
- `repeat s n` : `str, i64 -> str`. `s` concatenated `n` times; `""` when
  `n <= 0`. A result longer than 100,000,000 bytes is R005 in both engines
  (like the 10,000,000-element limit of `range`); without a fixed cap the
  interpreter tried to allocate terabytes before failing (review).
- `index xs x` : `list A, A -> opt i64`, A a primitive (`==` rules). `some`
  of the first position of `x`, `none` if absent.
- Pure, no capability; both engines identical (T-020); traps inside `f` as
  for `map`. Lambdas get regions as in other higher-order builtins
  (ADR-0039).
- `mapr`, `sortby`, `repeat`, `index` become reserved names (T005).

## Consequences
- The six long corpus programs can lose their helper functions; the corpus
  report is rerun after implementation and the ratio recorded.
- Four more builtin lines in llms.txt: they are paid for by trimming prose
  elsewhere, not by raising the budget (a decision for a person).
- Programs that used one of the four names as an identifier stop compiling
  (T005), as with every new builtin.
