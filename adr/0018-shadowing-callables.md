# ADR-0018: Shadowing locals and callables is T005

Status: Proposed
Date: 2026-11-15

## Context
SPEC §3.3 says `let` bindings are immutable and shadowing is an error (T005),
but it does not spell out what happens when a binding name is also a function,
record constructor or builtin. Prefix notation makes this ambiguity expensive:
if `let len 1` hid the builtin `len`, a later `len xs` could no longer be parsed
with the builtin's arity. Parameters and locals also share a namespace, so two
params with the same name must not both become silently visible.

## Decision
A parameter or `let` name that collides with an existing parameter/local, a
user function, a record constructor or a builtin produces `T005` "shadowed
binding". Top-level names are also unique file-wide (T-006/T-007 enforce this
with P001 when a function and a type, a builtin or a type name collide).

The parser reports one T005 per colliding declaration, including each colliding
parameter. The declaration is then **inserted** into the environment anyway, so
later expressions resolve the name as a variable and the parser does not
continue with a misleading arity error.

Files with a T005 are not formatted by `aleph fmt`: T005 is a parse-phase
error, and `fmt` only formats a source that parses cleanly. Leaving such files
untouched is intentional.

For example:

```
fn f x:i64 -> i64
 let x 1
 x
```

reports T005 for the `let`, and the result `x` refers to the `let` binding.
Likewise `let len 1` reports T005 and later `len` is a variable, not the
builtin.

Parameters are checked in declaration order. A colliding parameter is still
inserted, so `fn f len:i64 -> i64` reports T005 and the body sees `len` as the
parameter. If several parameters collide, each gets its own T005.

## Consequences
- Name resolution stays deterministic and arity-directed, and parsing recovers
  after T005 instead of emitting cascading P002 errors.
- Names are intended to be unique: reusing a param/local/callable is always a
  diagnostic, never a way to introduce a second meaning.
- The trade-off is that after T005 the parser deliberately follows the new
  binding rather than the original callable. Downstream phases still reject the
  file because a T005 diagnostic exists, so the recovery only affects which
  further errors are reported.
- The parser reports T005 directly, alongside P001-P003.
- A future scoped-binding feature would need a new ADR that supersedes this
  one.
