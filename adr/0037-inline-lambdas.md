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

## Amendments
Made while implementing T-045..T-047 (2026-11-19), where the decision above
was silent:

- `try` inside a lambda body is `T007` (no enclosing result function inside a
  lambda): an early return out of a callback has no defined target, and the
  lambda's result type may still be unresolved when its body is checked.
- `lam` anywhere but a function-typed builtin parameter is `P001` with
  `expected: "lam only as function argument of filter fold map map2"` (the
  list is derived from the builtin table).
- A binder is scoped to its lambda body, exactly like a `match` binder: two
  sibling lambdas may reuse a name, and a later `let` may reuse it.
- The interpreter captures only the outer names the body reads, by value.
  Values are immutable, so this is observably the same as capturing the
  whole environment.
- Until T-048, `aleph build` rejects a program with `lam` in a `let`, `do` or
  result line with `X001` ("not implemented") at the first lambda, with its
  node id, before emitting any C. Examples are not compiled, so `lam` in
  `ex` lines does not block a build.

Made while implementing T-048..T-049 (2026-11-19):

- Runtime layout: `AlFn` is `{code, name, kind, captured, ncaptured}`;
  `code` has type `AlFnCode = AlValue (*)(const AlFn *self, AlValue *args,
  size_t n)`, and `map`/`filter`/`fold`/`map2` call `fn->code(fn, args, n)`.
  Static `@f` values are `{..., NULL, 0}`. A lambda value has kind
  `AL_FN_LAM`, name `"lam"`, and its captured values copied into the arena by
  `aleph_fnval_lam` each time the `lam` node is evaluated (no allocation of
  the array when nothing is captured). Two lambda values are equal only when
  they are the same `AlFn` (unreachable from Aleph: function values are not
  comparable).
- Each `lam` node `n<id>` becomes `aleph_lam_n<id>` (the `AlFn` code: fast
  body with unboxed binders and captures, ADR-0027) and a cold
  `aleph_lamslow_n<id>` (instrumented body). Captures are the names the body
  reads minus the names bound inside it (its binders, nested binders, match
  binders), sorted; T005 keeps those apart from every visible outer name.
- Depth accounting mirrors `interp::call_lam`: `aleph_lam_enter` checks call
  and evaluation depth (R004 at the current location, the higher-order call)
  and then grows only the call depth; the body's own frames are counted by
  the body. A lambda body is not part of the enclosing function's frame
  (temporaries, static maximum depth); evaluating the `lam` node itself is
  one frame.
- `aleph explain` lists the lambdas of a body in pre-order as `lambdas: a
  function of x [and y] returning BODY` (text, one line each) and a
  `lambdas` string array (JSON), both omitted when the body has none, so the
  ADR-0023 output of lambda-free programs is unchanged. Lambdas in `ex`
  lines are not listed (like `calls`).
- `aleph edit`: a lambda and its body are expression nodes (binders are not
  nodes). A replacement fragment in the function position of a higher-order
  builtin is checked as `[CALLEE fragment 0 ..]` instead of `[fragment]`, since
  `lam` only parses there and takes its binder count from the callee.
- (review round 1) In a builtin call with a lambda argument, the expected
  type of the call is unified with the builtin's result type before the
  arguments are checked, so `map lam x ok x xs` (expected `list res i64 str`)
  and `fold lam a x .. ok 0 xs` type-check instead of T010. Calls without a
  lambda are unchanged. A lambda with no captures is one static `AlFn` in
  generated C (no arena allocation per evaluation).
- (review round 2) Order of checking a builtin call with a lambda: plain
  arguments, then the expected type (unify; a conflict rolls back and is
  reported at the call as before), then `ok`/`err`/`none` arguments, then
  lambdas. Unifying before the plain arguments moved mismatch errors into
  lambda bodies.
