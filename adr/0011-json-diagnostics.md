# ADR-0011: All diagnostics are structured JSON

Status: Accepted
Date: 2026-11-12

## Context
Prose error messages must be interpreted by the model and waste tokens in correction loops.

## Decision
Diagnostics follow SPEC §9: one JSON object per line with code, location, node id, expected/found and an optional certain fix. Codes are stable and documented in `spec/diagnostics.md`. Human-readable text is produced only by a separate renderer.

## Consequences
- Tests compare diagnostics as golden `.diag` files.
- Codes are an API: never reuse a code with a different meaning.
