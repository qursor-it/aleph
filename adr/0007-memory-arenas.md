# ADR-0007: Memory managed with arenas, no garbage collector

Status: Accepted
Date: 2026-11-12

## Context
GC adds runtime and unpredictable pauses. Rust-style ownership is powerful but models struggle with borrow checking. Values in Aleph are immutable.

## Decision
- v0.1: one arena per program run, freed at exit. Simple and fast for short-lived agent programs.
- v0.2: scoped regions per call; values returned from a call are copied (or promoted) to the caller region. Dangling references are impossible because there are no user-visible references.
- Ownership/borrowing is not planned; revisit only with benchmark evidence.

## Consequences
- Long-running programs may grow memory in v0.1; acceptable for the first niche (short agent scripts).
- Tlön exposes `aleph_alloc(arena, size)`; generated C never calls `free`.
