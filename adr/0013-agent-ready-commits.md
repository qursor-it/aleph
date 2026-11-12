# ADR-0013: Agent-ready commits on top of git

Status: Accepted
Date: 2026-11-12

## Context
The whole toolchain should be agent-ready, not only the language. Commits today are human prose plus line diffs. Replacing git now would block adoption.

## Decision
- Keep git. Every commit carries structured trailers: Intent, Task, Changed, Capabilities, Spec, Verified, Agent (format in AGENTS.md).
- Later (v0.3+): `aleph diff` produces a semantic diff on AST nodes, including capability changes, stored as a git note or trailer; the human-readable message is generated from it.
- Capability changes are always called out, since they are the human approval point.

## Consequences
- History is machine-queryable (`git log --grep "Task: T-007"`).
- Agents must never squash or rewrite history.
