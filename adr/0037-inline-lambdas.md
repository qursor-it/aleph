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
  k = 1; `fold`, `map2`: k = 2). Elsewhere: `P001` with `expected: "lam only
  as function argument of filter fold map map2"` (list derived from the
  builtin table). `@f` stays valid where it is valid today.
- Typing. A builtin call with a lambda argument is checked in this order:
  1. plain arguments (non-lambda, containing no `ok`/`err`/`none` anywhere);
  2. the call's expected type, unified with the builtin's result type (a
     conflict rolls back and is reported at the call);
  3. the other non-lambda arguments (`[ok 0]`, `if c ok 0 err e`, a nested
     call), which thus see the expected type as with a named helper;
  4. the lambdas, with binder types from the instantiated signature (`map`,
     `filter`: A from the list; `fold`: B from `init`, A from the list;
     `map2`: A and B from the lists). The body's type gives `B` for `map`,
     `T` for `map2`, must be `bool` for `filter`, `B` for `fold`.
  Calls without a lambda are checked as before. Limit: an `ok`/`err` element
  type that only a lambda body could supply (`map lam x else x 0 [ok 1]`) is
  `T010`.
- `try` inside a lambda body is `T007`: an early return out of a callback
  has no target.
- Names: binders follow T005 (no reuse of a visible parameter, local,
  function, constructor, builtin or outer binder name) and are scoped to the
  lambda body like `match` binders: sibling lambdas and later `let`s may
  reuse them.
- Effects: a lambda body may call effectful builtins only where the
  enclosing function may; `ex` lines may use lambdas.
- Equality: function values stay non-comparable; `show` of a function is a
  type error as today.
- Printing: canonical text is `lam x y BODY` with single spaces. `aleph
  explain` lists a body's lambdas in pre-order as `lambdas: a function of x
  [and y] returning BODY` (text) and a `lambdas` string array (JSON), both
  omitted when there are none (lambdas in `ex` lines are not listed).
- `aleph edit`: a lambda and its body are expression nodes (binders are
  not). A fragment in the function position of a higher-order builtin is
  checked as `[CALLEE fragment 0 ..]`.
- Interpreter: a function value is a target plus the captured values of the
  outer names its body reads (by value; values are immutable, so this equals
  capturing the environment).
- Native: `AlFn` is `{code, name, kind, captured, ncaptured}` with `code :
  AlValue (*)(const AlFn *self, AlValue *args, size_t n)`; the higher-order
  builtins call `fn->code(fn, args, n)`. A lambda has kind `AL_FN_LAM` and
  name `"lam"`; static `@f` values have no captures. Each `lam` node
  `n<id>` becomes `aleph_lam_n<id>` (fast body, unboxed binders and
  captures, ADR-0027) and a cold `aleph_lamslow_n<id>`.
  Captures are the names the body reads minus those bound inside it, sorted,
  copied into the arena by `aleph_fnval_lam` each time the node is evaluated;
  a lambda without captures is one static `AlFn`.
- Depth: `aleph_lam_enter` (as `interp::call_lam`) checks call and
  evaluation depth (R004 at the higher-order call) and grows only the call
  depth; the body counts its own frames and is not part of the enclosing
  function's frame.

## Consequences
- `lam` becomes a keyword (a program that uses it as a name stops compiling).
- `llms.txt` grows by a few lines; the rule "one result expression" also
  holds for lambda bodies (no `let` inside a lambda).
- The runtime `AlFn` layout and the callback signature change; every
  generated wrapper and `tlon/test_tlon.c` need updating (one task).
- Success is measured, not assumed: after implementation the HumanEval part
  of the token corpus is rewritten with lambdas and counted again; the
  estimate above (0.81) must be confirmed on compiled, test-passing code.

## History
Implemented in T-045..T-049 (19 Nov 2026). The rules on checking order,
`try`, binder scope, captures, the `AlFn` layout, depth, `explain` and
`edit` were added during implementation and three review rounds, then
folded into the Decision above (20 Nov 2026); see git history for the
round-by-round amendments.
