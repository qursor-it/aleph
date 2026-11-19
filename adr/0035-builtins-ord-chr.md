# ADR-0035: Builtins `ord` and `chr`

Status: Proposed
Date: 2026-11-19

## Context
T-044: the 18 Nov pilot found a model rebuilding character codes from an
alphabet string and failing (HumanEval 66). Aleph has no way to go between a
character and its code. Strings are valid UTF-8 (SPEC §4; `read` rejects
invalid data), `count` counts bytes, `upper`/`lower` are ASCII only.

Options (score 1-10):
- `ord s -> opt i64` (scalar value of the first character) and
  `chr n -> opt str` (one-character string), both total via `opt`. Works for
  all of Unicode, no new type, no trap. **8**
- Byte-level `byte s i -> opt i64` / `frombyte n -> opt str`: simpler, but
  non-ASCII text gives UTF-8 fragments, and `frombyte` could build invalid
  strings. **5**
- A `char` type: a new primitive in every layer for one use. **3**

## Decision
- `ord s` : `str -> opt i64`. `some` of the Unicode scalar value of the
  first character of `s` (decoded from UTF-8); `none` if `s` is empty. The
  rest of `s` is ignored.
- `chr n` : `i64 -> opt str`. `some` of the one-character string whose
  scalar value is `n` (UTF-8, 1 to 4 bytes); `none` if `n` is negative, above
  `0x10FFFF`, or a surrogate (`0xD800`..`0xDFFF`). `chr 0` is the one-byte
  string `"\u0000"`, as `json` strings already allow.
- Pure, no capability, no trap. Both engines identical (T-020).
- For every valid `n`, `ord else chr n ""` is `some n`.

## Consequences
- `ord` and `chr` become reserved names (T005).
- Caesar-style shifts are `else chr + 3 else ord c 0 ""`; per-character work
  splits the string into characters with `split s ""` (already in the
  language, unchanged by this ADR) and maps over them.
- `llms.txt` grows by two builtin lines (ADR-0026 budget).
