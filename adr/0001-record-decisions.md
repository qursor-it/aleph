# ADR-0001: Record architecture decisions as ADRs

Status: Accepted
Date: 2026-11-12

## Context
Aleph is built largely by autonomous agents in fresh sessions with no memory. Decisions must survive between sessions and be checkable.

## Decision
Every architectural decision is a file `adr/NNNN-title.md` with sections Status, Date, Context, Decision, Consequences. Status is one of Proposed, Accepted, Superseded by ADR-NNNN, Rejected. Agents may create Proposed ADRs; only humans move an ADR to Accepted or Rejected.

## Consequences
- Agents read ADRs before working on a related task.
- Proposed ADRs are followed provisionally until a human reviews them.
- History of reasoning is preserved in the repo, not in chat logs.
