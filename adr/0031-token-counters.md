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
  - `aleph`: tokens from the Aleph lexer (SPEC §2). On a lexical error,
    asked for alone, the lexer's diagnostics are reported (exit 1); with
    `all` its count is `-` (text) / `null` (JSON) and the others still count.
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

## Amendments
- 2026-11-20, T-039: `aleph tokens FILE [--tokenizer NAME|all] [--json]`
  also accepts a repeatable `--vocab PATH`. Each path loads a local BPE
  vocabulary and adds a counter named `bpe:<file stem>` (for example
  `bpe:o200k_base`); the counter is part of `all` and selectable by name.
  The format is detected from the file contents:
  - tiktoken (`base64token rank` lines, as in `o200k_base.tiktoken`),
    byte-level BPE by rank (lowest rank merges first). The pre-tokenizer is
    chosen from the entry count, not the file name: around 200k entries use
    o200k, around 100k use cl100k, around 50k use the GPT-2 regex
    (r50k/p50k). Files under 1000 entries are test fixtures and must be named
    `o200k_base`, `cl100k_base`, `r50k_base` or `p50k_base`; for real-sized
    vocabularies a name that disagrees with the size is an error, as is a
    count that matches none. A whole piece that is one token counts once.
    The regex used is named in the counter's `method`.
  - Hugging Face `tokenizer.json` with `"model": {"type": "BPE", "vocab",
    "merges"}` (merges as `"a b"` strings or `["a", "b"]` pairs) and a
    byte-level pre-tokenizer, using GPT-2's byte-to-unicode mapping; merge
    priority is the merge index and added/special tokens are ignored.
    `model.ignore_merges` is honoured (a whole piece already in the
    vocabulary counts once) and `ByteLevel.add_prefix_space` prepends one
    space at the start of the text when it does not already begin with a
    space.
  - `pre_tokenizer` may be a bare `ByteLevel` with `use_regex` (default true,
    giving the GPT-2 regex) or a `Sequence[Split{Regex}, ..., ByteLevel]`
    with `use_regex: false`. Only four exact Split patterns are reproduced by
    hand: cl100k, o200k, GPT-2 and Qwen2 (cl100k with one digit per piece).
    Any other pattern is a usage error on stderr (exit 2) naming the file and
    the first 80 characters of the pattern. A Split behaviour other than
    `Isolated`, a Split after ByteLevel, a bare ByteLevel with
    `use_regex: false` (which would treat the whole file as one piece), an
    unsupported component, or any pre-tokenizer we cannot reproduce is also
    an error: the counter never silently treats the file as one piece.
    Split patterns must be written as `{"Regex": ...}`; a bare string is
    rejected. Chained Split pre-tokenizers (several Split components, as in
    DeepSeek) are rejected with "chained Split pre-tokenizers (e.g. DeepSeek)
    are not supported": their patterns cannot be verified offline.
    `add_prefix_space: true` inside a Sequence is also rejected (HF applies it
    per Split piece; no target tokenizer uses it).
  - The normalizer may be absent/null or NFC (a single NFC or a Sequence of
    NFC). NFC is not applied: the supported vocabularies do not need it and
    ASCII code is unaffected. Any other normalizer is an error.
  - Unicode categories in the hand-written regexes are approximated with
    `char::is_alphabetic`, `is_numeric` and `is_whitespace`; o200k case
    classes use `is_uppercase`/`is_lowercase` plus "other letter"
    (alphabetic but neither). Unusual combining marks or numeric characters
    can differ from the exact regex.
  An unreadable, unknown or malformed vocabulary is a usage error on stderr
  with exit code 2, naming the file. The `tokens` MCP tool accepts an optional
  `vocab` array of paths with the same meaning. Status remains Proposed.
- 2026-11-20, T-039 review: the counts are still unvalidated. Before they
  feed EXP-003 results, a person must cross-check at least one real
  `o200k_base.tiktoken`, `cl100k_base.tiktoken` or byte-level
  `tokenizer.json` against tiktoken/tokenizers and record the agreement.

