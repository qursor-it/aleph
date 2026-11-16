# ADR-0031: Local token counters for `aleph tokens`

Status: Proposed
Date: 2026-11-16

## Context
T-026 asks for `aleph tokens` with pluggable tokenizers, starting from a local
BPE file if one is provided, otherwise a stub plus an ADR. No BPE vocabulary
is in the repository, and adding one (or a tokenizer crate) would need a
network download and user approval (AGENTS.md dependency rule).

## Decision
- `aleph tokens FILE [--tokenizer NAME|all] [--json]`.
- Counters live in a table in `compiler/src/tokens.rs` (`name`, `method`,
  `count`); adding a real tokenizer means adding an entry:
  - `chars4` (default): `ceil(unicode chars / 4)`, the estimate already used
    for the llms.txt budget (ADR-0026);
  - `bytes`: UTF-8 bytes; `chars`: Unicode scalar values;
  - `words`: whitespace-separated words;
  - `aleph`: tokens from the Aleph lexer (SPEC §2); a lexical error is
    reported as the lexer's diagnostics (exit 1).
- Text output: one `NAME COUNT` line per counter. JSON output: one object
  `{"file", "tokens": [{"tokenizer","method","count"}]}`.
- An unknown `--tokenizer` is a usage error (stderr, exit 2) listing the
  known names.
- The MCP server exposes the same as the `tokens` tool (ADR-0030), default
  `all`.

## Consequences
- Counts are deterministic and dependency-free, but `chars4` is only an
  estimate of LLM tokens. Measuring real model tokenizers (design goal "few
  tokens") needs a BPE file and is left for v0.2; a BPE counter would read a
  local vocabulary file given by path, not download it.
