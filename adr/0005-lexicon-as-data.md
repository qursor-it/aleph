# ADR-0005: Lexicon is data, optimized against real tokenizers

Status: Accepted
Date: 2026-11-12

## Context
Token cost depends on the model's tokenizer and on context (line start, after space, after symbol). Choosing keywords by intuition is unreliable; also very short opaque aliases are reported to reduce generation accuracy.

## Decision
- Keyword, type and builtin spellings live in `spec/lexicon.toml`, separate from the grammar.
- The compiler reads spellings from one Rust module generated or kept in sync with the lexicon file (a test checks they match).
- Before v1.0 a tool scores candidates on target tokenizers (count in context, stability across tokenizers) and on generation accuracy with the benchmark.
- Training/fine-tuning always uses one standard lexicon; per-model profiles are only for in-context use.

## Consequences
- v0 spellings are provisional; code must not hard-code keyword strings outside the lexicon module.
