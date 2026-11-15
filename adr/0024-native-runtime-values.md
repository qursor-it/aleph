# ADR-0024: Native runtime values and semantics

Status: Proposed
Date: 2026-11-15

## Context
The C backend (T-018/T-019) needs a runtime representation whose observable
behaviour is byte-identical to the reference interpreter (`interp.rs`,
ADR-0021). Tlön v0.1 lives in `tlon/tlon.h` and `tlon/tlon.c`: C99, libc and
pthreads only, compiled with GCC or Clang. Checked arithmetic uses
`__builtin_*_overflow`, so plain C99-only compilers such as tcc are not
supported. This ADR fixes the ABI, memory model, trap mechanics and edge
semantics that generated C may rely on.

## Decision

### Values
One tagged struct, passed by value:

```c
typedef struct AlValue {
    AlTag tag;              /* UNIT I64 F64 BOOL STR LIST NONE SOME OK ERR RECORD FN */
    union {
        int64_t i64;
        double f64;
        bool b;
        const AlStr *str;
        AlList *list;
        AlBox *box;         /* SOME / OK / ERR payload */
        AlRecord *rec;
        const AlFn *fn;
    } u;
} AlValue;
```

`AlStr { len, data }` is UTF-8 bytes plus a length (the data is NUL-terminated
as a convenience, but embedded NUL is not meaningful); `AlList { len, items[] }`
and `AlRecord { info, n, fields[] }` use flexible array members, so a list or
record is one allocation. `AlRecordInfo { name, field_count }` is static data
owned by generated code. `AlFn { code, name, kind }` stores
`AlValue (*code)(AlValue *, size_t)` and the `AL_FN_USER` /
`AL_FN_CTOR` / `AL_FN_BUILTIN` kind; function equality compares kind and name
like `FnTarget`. Values are immutable; builtins return fresh lists and strings.

### Arena
`aleph_alloc(size)` is a single global bump allocator: every returned address
is explicitly 16-byte aligned (the chunk base is rounded up), chunks of at
least 64 KiB are obtained with `malloc`, and allocations larger than the
current chunk get their own chunk. All size arithmetic is checked; overflow
traps `R005`. It never frees; generated code never calls
`free`. Allocation failure (including arithmetic overflow in a size
computation) traps `R005` "allocation limit exceeded". `range` additionally
checks the checked difference `b - a` and traps `R005` above
`10_000_000`, before allocating.

### Traps and depth
`AlLoc { node, line, col }` is the source location of an AST node. A location
stack is maintained by the generated code through
`aleph_eval_enter(loc)` / `aleph_eval_leave()`; `aleph_call_enter(loc)` /
`aleph_call_leave()` account for user calls. Entering counts exactly like
`interp.rs`: evaluation traps when `eval_depth >= 20_000`; a call traps when
`call_depth >= 10_000` or `eval_depth >= 20_000`, then increments both
counters. Both traps are `R004` "stack overflow". `aleph_call_enter` also
checks a native stack guard based on the main thread's stack base and traps
`R004` when less than 1 MiB of the 256 MiB stack remains; this can fire earlier
than the interpreter only for pathological native frames.

`aleph_trap(code, msg)` flushes stdout, writes exactly one JSON line to stderr
with fields in the order `code`, `severity`, `file`, `line`, `col`, `node`,
`msg`, and exits 70. `file` comes from `aleph_set_file`, defaults to `""`, and
is escaped like `serde_json` (quotes, backslashes, `\b\f\n\r\t`, other C0
controls as `\u00XX`; non-ASCII is raw UTF-8). `node` is the current location's
id rendered as `nN`. `aleph_main_run` runs `main` on a pthread with a 256 MiB
stack, sets the C locale for numeric formatting, ignores `SIGPIPE`, and treats
the result like `run_main`: unit/ok -> 0; `err e` -> flush stdout, write `e`
raw if it is a string else its `show`, add LF to stderr, return 1. If the
pthread attributes or thread cannot be created, it writes one line to stderr
and exits 70 instead of running on the default stack.

`read` and `write` check the capability first using the full, length-aware
path; an uncovered NUL path traps `R003`. A covered path containing an embedded
NUL then returns `err "invalid input"` (matching Rust's filesystem calls).

### Arithmetic and comparisons
i64 `+ - *` and `sum` use `__builtin_*_overflow` -> `R001` "integer overflow".
`/` and `%` trap `R002` "division by zero" for a zero divisor and `R001` for
`INT64_MIN / -1` and `INT64_MIN % -1`; `neg INT64_MIN` is `R001`. C division
truncates toward zero, like Rust. f64 uses IEEE arithmetic. Equality is
structural (`NaN != NaN`); `< <= > >=` are IEEE for f64 (all false when NaN).

### Builtins
All non-higher-order SPEC §5.1 builtins are `aleph_b_<name>` and follow
ADR-0021 exactly: negative or past-end `at` -> `none`; `slice` clamps to
`[0, len]`; `sort` is ascending and f64 uses `total_cmp` semantics (`-0.0 <
0.0`, NaNs ordered after infinities by sign/payload); `split` with `""`
separates Unicode scalar values; `trim` removes Unicode `White_Space`; `upper`
and `lower` are ASCII-only; `count` is bytes; `int` is `-?[0-9]+` in i64 range;
`float` is `-?[0-9]+\.[0-9]+`; `print` does not flush; `eprint` flushes stdout
before and stderr after; `read` reads bytes, rejects invalid UTF-8 as
`err "invalid data"`, and `read`/`write` failures map `errno` to the
`io_error_text` words from ADR-0021. `args` returns the program arguments
without the executable name.

Capabilities use `aleph_set_uses` plus ports of `resolve_lenient` and
`Path::starts_with`: empty paths and `..` are rejected, an existing path
(including a symlink) must `realpath` successfully, otherwise the longest
existing ancestor is canonicalized and the remaining components appended, then
both sides are compared component-wise. A failure traps `R003` "capability
violation" before touching the filesystem.

### show / literal
`show` mirrors `interp::show`: integers decimal, floats Rust `{:?}`, booleans
`true`/`false`, a top-level string unquoted, lists `[a b]` with string elements
quoted and escaped by the canonical `\n \t \" \\` rules, `none`, `some X`,
`ok X`, `err X`, records `NAME f1 f2`, functions `@name`. `literal` is the same
but quotes a top-level string.

Rust `{:?}` for f64 is reproduced with a shortest round-trip search. Edge cases
are `0.0`, `-0.0`, `NaN`, `inf`, `-inf`. Otherwise a high-precision exact-ish
decimal expansion (`"%.800e"`, enough for every double) is rounded to `p = 1
.. 17` significant digits and parsed back with `strtod` until it reproduces the
original bits. This differs from plain `printf` only on exact halfway cases,
where it rounds away from zero to match Rust's `flt2dec` (for example
`137839762462415.625` prints `137839762462415.63`). The resulting digits use
scientific notation when the decimal exponent is `< -4` or `>= 16`
(`d[.ddd]e<exp>`, no `+`, no leading zeros) and positional notation otherwise,
appending `.0` when the value is integral.

## Consequences
- Generated C can construct values with the `aleph_*` constructors and call
  builtins directly; no printing or allocation logic lives in generated code.
- Output, traps, `show`, and `literal` are byte-identical between the
  interpreter and the native backend for the covered operations.
- Higher-order builtins (`map`, `filter`, `fold`) are implemented in the runtime
  (`aleph_b_*`) on top of `AlFn`; generated C supplies one static wrapper per
  referenced callable (T-019).
- The runtime keeps a single arena per process (ADR-0007); long-running
  programs grow memory until exit, which v0.1 accepts.
