# ADR-0054: Helpers run by a caller's examples need no examples of their own

Status: Proposed
Date: 2026-11-27
Supersedes: part of ADR-0022 (the E001 rule)

## Context
E001 asks every pure function except `main` for an `ex` line. In the dev
probe answers (experiments/devprobe; run14, run14-haiku, rep16a-c) the `ex`
lines of helper functions, those called by another function of the answer,
are 8-16% of all characters a model writes. They are paid on every write and
every read. Python has no such rule, and model-written Aleph is about as long
as the same model's Python (T-081, T-083). The examples of the caller already
run the helper, so its bugs still surface as E002 on the caller's example.

Options (score 1-10):
- **A function needs no `ex` when some example runs it**: called from an
  `ex` line, or transitively from the body of such a function (calls and
  `@f` references, lambdas included). A function nobody's examples reach
  still reports E001. **7**
- **Examples optional everywhere.** Loses the guarantee that every pure
  function is exercised. **3**
- **Keep as is.** **4**

## Decision
- E001 "missing examples" is reported for a function that is not `main`,
  not effectful, and not run by any example (the set above). Examples of a
  helper are still allowed and run.
- Only E001 changes; E002, `aleph test`, the engines and codegen do not.
- llms.txt: `(E001; unless main/effectful/run by a caller's ex)`.

## Consequences
- A failing helper shows up as E002 on its caller's example, one level
  removed; the helper keeps the precise message when it has its own `ex`.
- To be measured on the dev probe (several runs per arm) before acceptance:
  characters written, first-attempt pass rate, compile errors.
