# ADR-0009: Errors as values, strict evaluation, no undefined behavior

Status: Accepted
Date: 2026-11-12

## Context
Hidden control flow (exceptions) and undefined behavior make generated code hard to verify.

## Decision
- Fallible operations return `res T E`; `try` propagates. No exceptions.
- Strict evaluation, arguments left to right. Immutable values.
- i64 overflow and division by zero trap (R001, R002) with a JSON error and exit code 70. Bounds-checked access returns `opt`.

## Consequences
- Checked arithmetic costs some speed; acceptable. A wrapping operator may be added later via ADR.
