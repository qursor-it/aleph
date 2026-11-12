# ADR-0003: The AST is the source of truth; text has one canonical form

Status: Accepted
Date: 2026-11-12

## Context
Free formatting creates variants that cost tokens, confuse models and produce noisy diffs. Agents benefit from a single valid spelling of every program.

## Decision
- Every program has exactly one valid text: the canonical printing of its AST (layout, spacing, item order fixed by SPEC §3 and §10).
- `aleph check` rejects non-canonical text with F001 and returns the canonical text in the diagnostic; `aleph fmt` produces it.
- AST nodes carry stable ids (pre-order `n0..`), used by diagnostics and future node-level edits.
- No free comments. Intent is recorded with `doc` annotations.

## Consequences
- Round-trip invariants `print(parse(t)) == t` and `parse(print(a)) == a` must be tested.
- Changing the lexicon (keyword spellings) is a mechanical reprint: migrations are automatic.
- Humans read `aleph explain`, not the source.
