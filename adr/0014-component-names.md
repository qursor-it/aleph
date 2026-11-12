# ADR-0014: Component names from Borges

Status: Accepted
Date: 2026-11-12

## Context
The project needs stable names for its parts. Full project name: Aleph 4 Agents (other historical projects are called Aleph).

## Decision
From Borges, "Tlön, Uqbar, Orbis Tertius" and "The Aleph":
- **Aleph**: the language and compiler (`aleph` binary, `.aph` files). The point that contains everything.
- **Tlön**: the runtime, where programs live — arenas, builtins, sandbox, capability enforcement. Code directory `tlon/`.
- **Uqbar**: the registry of verified definitions by hash. You find it if you know what to look for.
- **Orbis**: the corpus pipeline — the society of agents that writes the encyclopedia of Aleph.

## Consequences
- ASCII spellings in code: `aleph`, `tlon`, `uqbar`, `orbis`.
- Do not use the `.ale` extension (used by the historical ALEPH).
