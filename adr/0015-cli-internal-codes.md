# ADR-0015: Internal/tool diagnostic codes use the `X` prefix

Status: Proposed
Date: 2026-11-15

## Context
SPEC §9 defines code prefixes for language phases: `L` lexical, `P` parse, `F` format, `T` type, `C` capability, `E` example, `R` runtime trap. The CLI needs to report conditions that are not language errors, such as a command that is recognized but not implemented yet. No existing prefix covers these tool/internal diagnostics.

## Decision
Reserve the `X` prefix for tool and internal diagnostics. Codes remain stable and are documented in the diagnostics table alongside the language codes. Initial code:

| Code | Meaning |
|---|---|
| X001 | command not implemented |

`X001` is emitted as a normal SPEC §9 JSON diagnostic with `severity` `error` and a location of `line` 0, `col` 0 when no source location applies.

## Consequences
- Language diagnostics keep their existing prefixes; tooling conditions cannot collide with them.
- New tool/internal conditions take the next free `X` code and never reuse an existing one.
- When a command gains a real implementation, it stops emitting `X001`.
