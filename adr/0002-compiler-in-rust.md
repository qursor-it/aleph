# ADR-0002: Write the compiler in Rust

Status: Accepted
Date: 2026-11-12

## Context
The compiler must be fast, distributable as a single binary, and easy for agents to write correctly. Candidates: Rust, OCaml, Zig.

## Decision
Rust stable, single crate `alephc` producing binary `aleph`. Dependencies limited to `serde` and `serde_json`; others require a Proposed ADR. Hand-written lexer and recursive-descent parser, no parser generators.

## Consequences
- Algebraic enums and exhaustive matching fit compiler code; models write Rust well.
- `cargo fmt`, `clippy -D warnings` and `cargo test` are the quality gate.
- Self-hosting is a long-term goal, not in v0.x.
