# ADR-0048: Builtin `okor` (an `opt` or `res` with a chosen error)

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-22

## Context
Tasks often say "return err X if the input is missing or invalid". Aleph can
default a value (`else e d`) and propagate an error (`try e`), but cannot
turn a `none` into an error, or replace an error message, in one step. The
dev probe answers (experiments/devprobe) show what models do instead:
- `jname` in runs 2, 3, 5 and the corpus `json-field`:
  `if has jstr v ok else jstr v "" err "bad"` (test, then unwrap with a dummy
  default);
- `parsedate`/`inventory` in run6: `if not isok int ms err "bad"` followed by
  `let mo else int ms 0` (parse twice);
- run3 `jname` and run5 `sumints` failed their hidden tests because
  `try jparse text` / `try int t` passed the builtin's own message
  ("json: ...", "invalid int") where the task asked for "bad" or the token.
Rust has the same need and spells it `ok_or` / `map_err`.

Options (score 1-10):
- **`okor x e`: `opt T, E -> res T E` and `res T X, E -> res T E`**
  (`some v`/`ok v` -> `ok v`; `none`/`err _` -> `err e`). One builtin for both
  shapes, like `else` already serves both; with `try` it gives
  `let n try okor int s "bad"`. Name from Rust's `ok_or`. **8**
- **`try e d`: a two-argument `try` with the error to use.** Shorter, but
  changes a keyword's arity (every `try` would need two arguments or the
  parser would need lookahead). **4**
- **Only better llms text.** Leaves the double parse and the failures. **2**

## Decision
- `okor x e` : `X, E -> res T E` where X is `opt T` or `res T U`; result
  `ok v` when x holds a value v, else `err e` (the original error, if any,
  is dropped). E is any type; with an expected type it checks like the
  argument of `err`.
- Pure, no trap; both engines identical (T-020); `okor` is reserved (T005).
- llms.txt line: `okor opt T|res T U,E->res T E (else err e)`, paid by
  trimming prose.

## Consequences
- "missing or invalid -> err X" is one expression:
  `okor jstr else get d "name" json nil "bad"`, `try okor int t t`.
- `try` keeps its meaning; programs using `okor` as a name stop compiling
  (none in the repository or in the HumanEval prompts).
