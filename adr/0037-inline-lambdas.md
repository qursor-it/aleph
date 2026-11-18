# ADR-0037: Inline lambdas `lam`

Status: Proposed
Date: 2026-11-18

## Context
The token pilot of 18 Nov (`experiments/pilot/tokens/` on `exp/protocols`)
counted 77 verified programs in Aleph and Python with real tokenizers. Aleph
bodies are shorter than Python's (median 0.74), but 39% of Aleph's tokens are
function signatures. Without anonymous functions or closures, every `map`,
`filter` or `fold` callback is a named, fully typed function, and anything
that needs a value from the enclosing function (a threshold, a prefix, a
second list) becomes an index-and-accumulator recursion. Aleph programs have
1.44 helper functions on average, Python 0.13.

Re-counting the HumanEval part of the corpus (`whatif.py`):

| Variant | Median Aleph/Python | Aleph shorter | Helpers per program |
|---|---|---|---|
| today | 1.23 | 20/61 | 0.98 |
| ADR-0036 builtins | 1.08 | 24/61 | 0.84 |
| builtins + lambdas | 0.81 | 42/61 | 0.20 |

Lambdas are the largest single lever found so far. Constraints: prefix
syntax, fixed arity, no parentheses (arity known before parsing an
expression), one canonical text per AST (ADR-0003), interpreter and native
byte-identical (T-020), no shadowing (T005).

## Options (score 1-10)

- **`lam x .. BODY`, binder count from the expected function type.**
  Function values only appear as the function argument of a higher-order
  builtin (`map filter fold map2`), whose signature fixes the arity (1 or 2),
  so the parser knows how many binders follow. One new keyword, no new
  punctuation, one token in common tokenizers. Body is one expression.
  Binders may read the enclosing parameters, lets, match binders and outer
  lambda binders (captured by value; values are immutable, so this is exact).
  **8**
- **Implicit parameters (`it`, or `a`/`b` for two-argument callbacks).**
  Fewest tokens, but nested lambdas are ambiguous and the names clash with
  user names; a canonical printer would need renaming rules. **5**
- **Partial application of named functions (`@f x` binds the first
  parameters).** Removes most index recursions without new body syntax, but
  each callback is still a named, typed helper, which is the cost being
  removed. **5**
- **`fn x:T -> R BODY` inside expressions.** Explicit types make checking
  trivial but cost most of what the change is meant to save. **3**

## Decision

`lam`, as in the first option:

- Grammar: `lam IDENT{k} expr`, allowed only as the function argument of a
  builtin whose parameter type is a function of arity k (`map`, `filter`:
  k = 1; `fold`, `map2`: k = 2). Elsewhere: P-error with the expected
  position. `@f` stays valid where it is valid today.
- Typing: the checker checks the non-function arguments first, then the
  lambda with its binder types taken from the instantiated signature (`map`
  and `filter`: A from the list; `fold`: B from `init`, A from the list;
  `map2`: A and B from the lists); the body's type gives `B` for `map` and
  `T` for `map2`, must be `bool` for `filter`, `B` for `fold`.
- Names: binders follow T005 (no reuse of a parameter, local, function,
  constructor or builtin name, including outer lambda binders).
- Effects: a lambda body may call effectful builtins only where the
  enclosing function may (same capability rules); `ex` lines may use lambdas.
- Equality: function values stay non-comparable; `show` of a function is a
  type error as today.
- Printing: canonical text is `lam x y BODY` with single spaces; `aleph
  explain` reads it as "a function of x and y returning …".
- Interpreter: a function value is a target plus captured values.
- Native: each lambda is lifted to a static C function taking its arguments
  and a pointer to the captured values; `AlFn` gains a captured-values array
  (static references keep it empty), allocated in the arena when the lambda
  expression is evaluated. Calls through `map`/`filter`/`fold`/`map2` pass the
  `AlFn` to the code. The fast path treats lambda bodies like function bodies.

## Consequences
- `lam` becomes a keyword (a program that uses it as a name stops compiling).
- `llms.txt` grows by a few lines; the rule "one result expression" also
  holds for lambda bodies (no `let` inside a lambda).
- The runtime `AlFn` layout and the callback signature change; every
  generated wrapper and `tlon/test_tlon.c` need updating (one task).
- Success is measured, not assumed: after implementation the HumanEval part
  of the token corpus is rewritten with lambdas and counted again; the
  estimate above (0.81) must be confirmed on compiled, test-passing code.
