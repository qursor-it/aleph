# ADR-0012: Content-addressed definitions (Uqbar)

Status: Proposed (target v0.3)
Date: 2026-11-12

## Context
Agents repeatedly regenerate the same functions. Version conflicts and lockfiles add friction. Unison shows that identifying definitions by the hash of their AST removes both problems.

## Decision
- Each function's identity = hash of its canonical AST with dependencies replaced by their hashes (names excluded).
- Uqbar is the registry of verified definitions: stored by hash, searchable by signature and `doc` intent.
- Before writing a function, an agent queries Uqbar; the cheapest code is code that is not generated.

## Consequences
- Requires stable canonical printing (ADR-0003) first.
- Not implemented before M3; the compiler should avoid design choices that make hashing hard (e.g. non-deterministic ids).
