# ADR-0010: Inline examples are mandatory and executed at check time

Status: Accepted
Date: 2026-11-12

## Context
Agents need fast, local, precise failure signals. Examples also document intent compactly.

## Decision
- Every function except `main` and effectful functions must have at least one `ex` line (E001).
- `aleph check` and `aleph test` run examples with the interpreter; failures produce E002 with expected and actual values.
- Property-based tests: v0.2.

## Consequences
- Every corpus program is self-testing, which makes Orbis verification cheap.
