# ADR-0021: Runtime semantics v0.1

Status: Proposed
Date: 2026-11-15

## Context
SPEC §5, §5.1 and §6 fix the language semantics but leave several edge cases
open: how `print` delimits lines, the exact `show` spelling, splitting on an
empty separator, negative list indexes, string-to-number parsing, the depth
and allocation limits, path resolution for capabilities, and the error text
returned by `read`/`write`. The interpreter
(T-013) needs one deterministic answer for each, shared later with Tlön and the
C backend.

## Decision
- **Lines.** `print` and `eprint` write their string argument followed by
  exactly one LF. A program that prints `"a"` twice therefore produces
  `a\na\n`. Before every write to stderr (`eprint` and the `main` `err`
  payload) stdout is flushed, and stderr is flushed immediately afterwards, so
  diagnostics and traps cannot overtake buffered program output.
- **`show`.** Integers are decimal (`-0` prints `0`); floats use Rust's `{:?}`
  (`1.0`, `1.5`, `inf`, `NaN`); booleans are `true`/`false`; a top-level string
  is returned unquoted. A list is `[e1 e2 ...]` with single spaces and no
  commas; string elements are quoted with the canonical escapes from
  `ast::escape_str`, other elements use the same spelling as above, and an
  empty list is `[]`. `show` is only well typed for primitives and lists of
  primitives (SPEC §5.1).
- **`split`.** With a non-empty separator, `split s sep` follows Rust
  `str::split`, including `split "" "," == [""]`. With the empty separator it
  returns one string per Unicode scalar value; `split "" ""` is `[]`.
- **`at`.** A negative index and an index at or past `len` both return `none`.
- **`int`.** Accepts an optional leading `-` followed by one or more ASCII
  digits and requires the value to fit in `i64`; otherwise it returns
  `err "invalid int"`. `+1`, `1.0`, the empty string and out-of-range values
  are invalid. Leading zeros are accepted (`"007"` is `7`).
- **`float`.** Accepts exactly `-?[0-9]+\.[0-9]+` and parses it with Rust's
  `f64` parser; everything else returns `err "invalid float"`. Exponent
  notation, a missing integer or fractional part (`1.`, `.5`) and `inf`/`NaN`
  spellings are invalid.
- **Range allocation.** `range a b` with `b > a` first computes `b - a` with
  checked subtraction. A difference above 10 000 000 traps with
  `R005` "allocation limit exceeded" instead of attempting the allocation;
  `a >= b` is still the empty list and subtraction overflow is R005.
- **Depth limits.** At most 10 000 nested user function calls are allowed
  (builtins and constructors do not count). Independently, total evaluation
  depth (every expression and call frame) is capped at 20 000. Exceeding either
  limit traps with `R004` "stack overflow".
  [`crate::interp::run_main`] executes on a thread with a 256 MiB stack; the
  evaluation budget was chosen so a function whose body is a 250-deep `+`
  chain recursing at its innermost node traps cleanly without a native stack
  overflow. The native backend mirrors both counters, but additionally checks
  a stack guard (ADR-0025); for pathological native frame sizes it may
  therefore report `R004` earlier than the interpreter. It never reports it
  later and never crashes with a native stack overflow.
- **Capability paths.** A path and a declared prefix are both resolved with
  `resolve_lenient`: `..` components are rejected; an existing path (including
  a symlink) must canonicalize successfully, so a dangling symlink is a
  capability violation; otherwise the longest existing ancestor is
  canonicalized and the remaining components are appended. The two resolved
  absolute paths are compared component-wise with `Path::starts_with`. This
  lets not-yet-existing write targets and nested missing read paths work
  inside a prefix while still resolving symlinks. TOCTOU races between the
  check and the filesystem operation, and hard links that point outside the
  prefix, are known v0.1 limitations: closing them needs `openat2`/`libc`,
  which the crate does not depend on.
- **`read`/`write` errors.** A failed filesystem operation returns
  `err TEXT` where `TEXT` is the `std::io::ErrorKind` as lowercase words.
  Explicit texts are `not found`, `permission denied`, `already exists`,
  `invalid input`, `invalid data`, `is a directory`, `not a directory`,
  `directory not empty`, `read-only filesystem`, `would block`,
  `unexpected eof` and `out of memory`; every other kind is `io error`.
  Capability failures are traps (`R003`), never `err` values.

## Consequences
- Output is byte-identical between the interpreter and the C backend if Tlön
  follows this ADR.
- `read` of a missing file (including under missing nested directories) inside
  a declared prefix returns `err "not found"`, because the longest existing
  ancestor is resolved and the missing suffix is appended; absence is an io
  error rather than a capability violation.
- The depth and range limits are deterministic, language-level traps; they are
  not implementation stack or allocator limits.
