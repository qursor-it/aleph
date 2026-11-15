# ADR-0025: C code generation strategy for the native backend

Status: Proposed
Date: 2026-11-15

## Context
T-017 fixed the Tlön runtime ABI and semantics (ADR-0024). T-018 needs a code
generator whose observable behaviour is byte-identical to the interpreter
(`interp.rs`, ADR-0021), including traps, evaluation order and the R004 depth
limits. The generator also needs a deterministic textual output so the same
source produces the same C.

## Decision

### Shape of the generated translation unit
One file contains, in order: `#include "tlon.h"`, one
`static const AlRecordInfo RI_<name>` per record, one
`static const AlLoc L_n<id>` per AST node (function and expression), forward
declarations for every generated function, one wrapper and one static
`const AlFn` per referenced function value, one definition per user function,
and an entry wrapper plus `main`.

A user function `fn f a:i64 b:i64 -> i64` becomes:

```c
static AlValue aleph_fn_f(AlValue l_a, AlValue l_b);
static AlValue aleph_fn_f(AlValue l_a, AlValue l_b) { ... }
```

Record constructors call `aleph_record(&RI_<name>, fields, n)` where `fields`
is a function-local `AlValue` array. Lists are built the same way with
`aleph_list`. Every builtin call is a direct `aleph_b_<name>(...)` call; no
builtin is reimplemented in generated code.

### Function values and higher-order builtins
`@f` resolves in the same order as `interp::fn_target`: user function, record
constructor, builtin. Each distinct used `f` gets exactly one static wrapper

```c
static AlValue aleph_ref_<kind>_<name>(AlValue *args, size_t n);
```

and one static value

```c
static const AlFn aleph_fnref_<kind>_<name> =
    {aleph_ref_<kind>_<name>, "<name>", AL_FN_<KIND>};
```

A `FnRef` expression is `aleph_fn_static(&aleph_fnref_<kind>_<name>)`; this
wraps the static `AlFn` without allocating. User wrappers perform
`aleph_call_enter(NULL)`/`aleph_call_leave()` around the generated call, which
mirrors `call_target` -> `call_user` at the higher-order call's node (NULL means
"use the current location", which is the enclosing `map`/`filter`/`fold`
call). Constructor and builtin wrappers do not enter a call frame, also
matching `call_target`. The runtime implements the loops:

- `aleph_b_map(fn, xs)` builds a list of `fn->code({x}, 1)`, left to right;
- `aleph_b_filter(fn, xs)` keeps `x` when `fn->code({x}, 1).u.b` is true;
- `aleph_b_fold(fn, init, xs)` folds `fn->code({acc, x}, 2)` left to right.

Because the higher-order builtin is evaluated inside its call expression's
`aleph_eval_enter` frame, the current location seen by a callback wrapper is
the higher-order call node (matching the interpreter's `node`/`span` arguments
to `call_target`); expression evaluations inside a user callback then push
their own locations, so an R002 inside the mapped function reports the body
node exactly like `aleph run`.

### Temporaries and evaluation order
Every expression node `n<id>` is evaluated into a function-local
`AlValue t_n<id>` declared at the top of its function. List literals call
`aleph_list_new(n)` and fill `u.list->items[i]`; record constructors use a
fresh arena list as their field array and pass `u.list->items` to
`aleph_record` (which copies the fields). No generated local array lives on the
C stack. Statements and the result expression are emitted in source order;
arguments, operands and list elements are emitted strictly left to right
(SPEC §5, ADR-0009). `if` becomes a C `if`/`else` that assigns the node's
temporary from the branch temporary, and `and`/`or` use a C `if` so only the
required operand is evaluated.

Generated C identifiers use an underscore pattern that Aleph identifiers
(`[a-z][a-z0-9]*`) cannot form: user parameters and lets are `l_<name>`,
temporaries are `t_n<id>`, the return slot is `aleph_ret_`, wrappers are
`aleph_ref_<kind>_<name>` and the epilogue label is `L_epilogue`. This makes a
user binding named `result`, `args`, or similar harmless.

### `try` and the function epilogue
Each generated function has a single `L_epilogue:` label and a local
`AlValue l_result`. The function result is evaluated last, assigned to
`l_result`, and then the label returns it. A `try` whose operand is `AL_ERR`
assigns the operand value (already `err` with the same payload) to `l_result`
and jumps to `L_epilogue`.

Before the jump, the generator emits one `aleph_eval_leave()` for every
evaluation frame that is open at that point (the `try` node and each enclosing
expression on the path, known statically from the emission depth). This mirrors
the interpreter, where the `Flow::Return` value unwinds through every `eval`
frame and decrements the depth on the way. The user-call frame itself is closed
by the call site's `aleph_call_leave()` after the generated function returns.

### R004 depth mirroring
`interp::eval` checks `eval_depth >= MAX_EVAL_DEPTH` and then increments for
*every* expression, including call expressions. `interp::call_user` checks the
call and evaluation limits and then increments both before running the body.
Generated code mirrors this exactly:

- every `emit_expr` starts with `aleph_eval_enter(&L_n<id>)` and ends with
  `aleph_eval_leave()`;
- a user call evaluates its arguments first, then calls
  `aleph_call_enter(&L_n<call id>)`, calls the generated function, and calls
  `aleph_call_leave()`.

`aleph_eval_enter` pushes the expression's location on the runtime location
stack and `aleph_eval_leave` pops it, so a trap raised by a builtin or checked
arithmetic uses the same node, line and column as the interpreter.
`aleph_call_enter` does **not** push a location: it only checks the call/eval
depth limits (and the native stack guard) and uses its `loc` argument if it
must trap; a NULL location means the current one. `main` is invoked through an
entry wrapper that performs `aleph_call_enter`/`aleph_call_leave` with the
`main` function's location, matching `interpret_main`'s `call_user`.

### Native stack guard
The language-level R004 limits are not a guarantee that a pathological tree of
huge C frames fits in the 256 MiB main-thread stack. `aleph_main_run` records
the main thread's stack base in `main_thread`; every `aleph_call_enter`
compares the address of one of its locals with that base and traps `R004`
"stack overflow" when less than 1 MiB of the nominal stack remains. The guard is
disabled outside the main thread (for example in the runtime unit tests).
Consequently the native backend may trap R004 *earlier* than the interpreter
only for pathological frames, never later and never with a SIGSEGV.

### Literals
- Integers use `INT64_C(<value>)`, with `-INT64_C(<magnitude>)` for negative
  values and `INT64_MIN` for the minimum (whose magnitude is not a valid
  positive `int64_t` literal).
- Floats are parsed with Rust's `str::parse::<f64>` at generation time and
  emitted as their exact `uint64_t` bit pattern via
  `aleph_f64_bits(UINT64_C(0x...))`. This preserves `-0.0`, infinities and
  NaN without relying on the C compiler's decimal parser.
- Strings use C string literals. `"` and `\` are escaped, every other
  non-printable or non-ASCII UTF-8 byte becomes a three-digit octal escape, and
  every `?` is escaped as `\?` so two of them can never form a trigraph.
  `aleph_str_n` receives the explicit byte length.

### Field access
`interp` finds a field by name in the record type's declaration order.
Generated code uses the same order: `check::CheckOutput::expr_types` provides
the resolved type of the base expression, the record name selects
`CheckOutput::records`, and the field position becomes the index into
`AlValue.u.rec->fields`.

### Internal consistency
`CheckOutput::expr_types` is resolved again with the final substitution after
all functions have been checked. Whenever codegen needs a type it requires a
concrete `i64`, `f64`, record, `opt` or `res`; a remaining inference variable
or error type is an internal inconsistency and is reported as `X006` "internal
compiler error" instead of guessing a C operation.

### Build pipeline (`aleph build FILE -o OUT`)
1. Read, lex/parse, type check and capability check (no F001, no examples).
2. Reject a missing `main` with `X003`.
3. Generate C (this ADR), including function values and the higher-order
   builtins. `X005` is no longer emitted; it remains reserved in the
   diagnostics table.
4. Write `gen.c`, `tlon.c` and `tlon.h` (embedded with `include_str!` from
   `tlon/`) into a fresh temporary directory.
5. Run `$CC -std=c99 -O2 -pthread -o OUT gen.c tlon.c` using `CC` when set,
   otherwise `cc`. `CC` must name the compiler executable (a bare path, not a
   command line). A compiler failure forwards the compiler's stderr and emits
   `X004`; success writes nothing to stdout and exits 0. The temporary build
   directory is created with mode 0700 on Unix.
6. Remove the temporary directory.

A missing `-o` is a usage error (exit 2).

### Generated `main`
`int main(int argc, char **argv)` calls `aleph_set_file(FILE)`,
`aleph_set_uses` with a static `AlUse` table (filesystem capabilities only;
other v0.2 capabilities are ignored until they have builtins),
`aleph_set_args`, and then `aleph_main_run(aleph_entry, argc - 1, argv + 1)`.
`FILE` is exactly the path passed to `aleph build`, so trap JSON matches
`aleph run` on the same path.

## Consequences
- All v0.1 expressions, including `map`/`filter`/`fold` and function values,
  build natively with interpreter-identical output and trap locations.
- The generator is O(nodes) and deterministic: no hash iteration, no
  timestamps, no compiler-dependent formatting.
- Because expression temporaries are function-local and immutable after
  assignment, generated code needs no naming discipline beyond the Ast node
  ids (which are already deterministic), and C scoping cannot change Aleph
  semantics.
- The runtime remains the only place that implements values, traps,
  capabilities and builtins; generated C is glue.
