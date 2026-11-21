# ADR-0046: Builtin `tof` (i64 to f64)

Status: Proposed
Date: 2026-11-21

## Context
Aleph has no conversions (SPEC §4) and no builtin from `i64` to `f64`. The
only route is text: `else float cat show n ".0" 0.0`, since `float "3"` is an
error (a float literal needs a fraction). Means, medians, averages and
ratios over counts all need it: 5 of the 156 translated HumanEval headers
mix `i64` and `f64`, and in the 21 Nov dev probe (`experiments/devprobe/`)
the model wrote that exact helper twice (`mean`, `median`) and named it
`tof` itself. The agent's own reference solution first wrote `float show x`, which
silently gave the default.

Options (score 1-10):
- **`tof n` : `i64 -> f64`, the nearest double (exact up to 2^53).** One
  line; the name the model chose; total, no trap. **8**
- **`float` overloaded on `i64`.** `float` returns `res f64 str` for
  strings; a different result type per argument type is a trap for readers.
  **4**
- **Also `toi`/`round` (f64 -> i64).** Needs a rule for NaN and out-of-range
  values; no evidence yet. Left for a later ADR. **4**

## Decision
- `tof n` : `i64 -> f64`, IEEE round-to-nearest-even conversion (C cast
  `(double)`, Rust `as f64`: identical). Pure, no trap.
- `tof` becomes a reserved name (T005).
- llms.txt: `tof i64->f64`.

## Consequences
- Averages are `/ fold ... tof len xs`.
- f64 to i64 stays unavailable until evidence asks for it.
