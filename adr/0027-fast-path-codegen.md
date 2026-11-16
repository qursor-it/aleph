# ADR-0027: Unboxed fast path with exact R004 fallback

Status: Proposed
Date: 2026-11-16

## Context
ADR-0025 mirrors `interp.rs` node by node: every expression is wrapped in
`aleph_eval_enter`/`aleph_eval_leave` and every value is a boxed `AlValue`,
including `i64`, `f64` and `bool` temporaries. Primitive operations are
out-of-line calls into `tlon.c`, a separate translation unit, so nothing is
inlined. On `fib35` the native binary was ~75x slower than the C reference and
slower than CPython, which defeats the point of a native backend.

The per-node accounting exists only to make `R004` (evaluation depth >= 20 000)
trap at exactly the same node as the interpreter. That precision is a language
requirement (ADR-0021) and must be kept.

## Decision
Every user function is emitted twice:

- `aleph_slow_<name>`: the ADR-0025 body, unchanged, marked `AL_COLD`.
- `aleph_fn_<name>`: a fast body. `i64`, `f64` and `bool` values are C
  `int64_t`/`double`/`bool`; every other type stays an `AlValue`. Function
  parameters and results use the same unboxed representation, so calls between
  fast bodies never box. Values are boxed only where they enter the runtime
  (builtin arguments, list/record/opt/res payloads, `@f` wrappers).

The fast body starts with `if (!aleph_fast_ok(M)) return aleph_slow_<name>(...)`,
where `M` is computed at compile time: the deepest expression nesting in the
body, and `depth + 1` for every user call or higher-order builtin (the frame the
interpreter's call check sees). If `eval_depth + M < 20 000` no node of this
activation can reach the evaluation limit, so it needs no per-node checks. If it
could, the instrumented body runs and traps at the exact node, as before.

Inside the fast body:

- a user call at nesting `d` does `aleph_fast_call_enter(loc, d + 2)`: native
  stack guard and call-depth check (same order and location as
  `aleph_call_enter`), then advances `eval_depth` by the `d + 2` frames the
  interpreter would have open when the callee body starts;
- a higher-order builtin (`map`, `filter`, `fold`) advances `eval_depth` by
  `d + 1` around the call, so callback wrappers see the interpreter's depth;
- each activation owns one slot on the location stack (`aleph_loc_push`/`pop`);
  `aleph_here(&L_n<id>)` sets it right before every operation that can trap or
  reads the current location (checked i64 arithmetic, builtins), so R001, R002,
  R005 and capability traps report the same node as the slow path;
- checked i64 arithmetic, boxing and float literals are `static inline`
  helpers in `tlon.h` (`aleph_fast_add`, `aleph_box_i64`,
  `aleph_f64_from_bits`, ...), with the same trap codes and messages.

`f64` comparisons and `==` use C operators, which are IEEE like
`aleph_values_equal`/`aleph_lt` (all ordered comparisons false with NaN,
`NaN != NaN`, `-0.0 == 0.0`).

## Consequences
- `fib35`: 1.71 s -> 0.115 s on the same x86-64 machine (CPython 1.1 s, C
  reference 0.02 s). The remaining gap to C is the call bookkeeping (depth
  counters, location slot, stack guard), kept on purpose for exact traps.
- Generated C roughly doubles in size; the slow copy is cold code.
- Output and traps stay byte-identical to the interpreter. The differential
  test covers this, plus three new goldens at the depth limit:
  `run_trap_depth_eval` (eval limit reached through recursion),
  `run_trap_depth_map` (through a `map` callback) and `run_depth_near_limit`
  (largest depth that does not trap, then the first one that does, so
  execution crosses from fast to slow bodies).
- `tlon.c` now exports its depth and location state as `aleph_g_*` globals
  for the inline helpers; nothing else in the runtime ABI changes.
