# ADR-0015: Internal/tool diagnostic codes use the `X` prefix

Status: Proposed
Date: 2026-11-15

## Context
SPEC §9 defines code prefixes for language phases: `L` lexical, `P` parse, `F` format, `T` type, `C` capability, `E` example, `R` runtime trap. The CLI needs to report conditions that are not language errors, such as a command that is recognized but not implemented yet. No existing prefix covers these tool/internal diagnostics.

## Decision
Reserve the `X` prefix for tool and internal diagnostics. Codes remain stable and are documented in the diagnostics table alongside the language codes. Initial codes:

| Code | Meaning |
|---|---|
| X001 | command not implemented |
| X002 | cannot read file |
| X003 | no main function |
| X004 | c compiler failed |
| X005 | reserved (no longer emitted) |
| X006 | internal compiler error |

`X001` is emitted as a normal SPEC §9 JSON diagnostic with `severity` `error` and a location of `line` 0, `col` 0 when no source location applies. `X002` uses the same shape when a file cannot be read, and `X003` when `aleph run`/`aleph build` has no `main` function to execute. `X004` reports a failed C compiler invocation (its stderr is forwarded to stderr), `X005` is retained as a reserved code: it was used for native-backend constructs that were not implemented yet, and T-019 removed its last emitter. It must not be reused for a different meaning. `X006` reports an internal compiler inconsistency, such as an expression type that is still an inference variable after checking; it is a bug, not a user error.

## Consequences
- Language diagnostics keep their existing prefixes; tooling conditions cannot collide with them.
- New tool/internal conditions take the next free `X` code and never reuse an existing one.
- When a command gains a real implementation, it stops emitting `X001`.
