# ADR-0008: Capabilities declared, denied by default

Status: Accepted
Date: 2026-11-12

## Context
Code written by agents and read by nobody must not have implicit power. Humans approve at the boundaries, not line by line.

## Decision
- Modules declare capabilities with `use` lines (fs.read, fs.write with path prefixes; net.host, env, time in v0.2). Default: nothing except stdout/stderr and `args`.
- The checker rejects effectful calls without a covering capability (C001) and literal paths outside declared prefixes (C002).
- Tlön enforces at run time (R003) for non-literal arguments.
- `aleph explain` always lists capabilities first.

## Consequences
- A diff in capabilities is the most important review signal (see ADR-0013).
- The WASM target will map capabilities to WASI preopens.
