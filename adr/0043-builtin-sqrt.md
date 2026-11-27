# ADR-0043: Builtin `sqrt`

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-20

## Context
Aleph has no square root: the EXP-003 `points` program carries a 40-step
Newton loop and a helper (ratio 1.49 against Python, whose reference mirrors
the loop for fairness). Distances, norms, standard deviations and primality
bounds are common in agent tasks and HumanEval. IEEE 754 requires `sqrt` to
be correctly rounded, and both C's `sqrt` and Rust's `f64::sqrt` are, so the
two engines agree bit for bit.

Options (score 1-10):
- **`sqrt x` : `f64 -> f64`, IEEE semantics (NaN for negative input,
  `sqrt -0.0 = -0.0`, `sqrt inf = inf`).** One line, no trap, identical in
  both engines. **8**
- **`sqrt x` : `f64 -> opt f64`, `none` for negative input.** Safer in
  spirit, but every use needs `else`, and NaN is already a value `f64`
  programs handle (`show`, `sort`, ADR-0024). **5**
- **A math module (`pow exp log sin ...`).** More than the evidence asks
  for; each function is a line of llms.txt budget. **3**

## Decision
- `sqrt x` : `f64 -> f64`, the IEEE 754 square root (correctly rounded):
  NaN for negative or NaN input, `-0.0` for `-0.0`, `inf` for `inf`.
- Pure, no capability, no trap; interpreter (`f64::sqrt`) and Tlön (C99
  `sqrt`; `aleph build` and the runtime tests now pass `-lm`) identical.
- `sqrt` becomes a reserved name (T005).

## Consequences
- `points` can drop its Newton loop; the Python reference keeps its own loop
  only if the Aleph version keeps one (the corpus compares like with like).
- One more builtin line in llms.txt, paid by trimming prose.
