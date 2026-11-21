# ADR-0047: Builtin `fields`

Status: Proposed
Date: 2026-11-22

## Context
Splitting text into words is the most common text operation in agent tasks,
and Python spells it `s.split()`: any run of whitespace separates, no empty
items. Aleph has only `split s SEP`, so every program writes
`filter lam w > count w 0 split s " "`, and text with newlines needs
`split join split text "\n" " " " "` first. In the EXP-003 corpus
`text-top-words` is still the longest program against Python (1.50, T-070)
for this reason; the dev probe tasks `wordcount`, `wordlen` and `initials`
(tasks2) and the run2/run3 answers spell the same filter, and two answers
compared words with `""` by hand.

Options (score 1-10):
- **`fields s` : `str -> list str`, split on runs of ASCII whitespace
  (space, `\t`, `\n`, `\r`, `\v`, `\f`), no empty items.** Python's
  `split()` meaning under Go's name (`strings.Fields`); one line; ASCII like
  `upper`/`lower`/`trim`. **8**
- **The same builtin named `words` (Haskell).** The better-known name for
  this in functional code, but `words` is a parameter of the translated
  HumanEval problem 158 (its prompt and hidden tests would have to change)
  and a function in three goldens and the SPEC §12 example. **6**
- **`split s ""` meaning whitespace.** Already means Unicode characters.
  **1**
- **Leave it to `filter`.** Costs every text program a dozen tokens. **3**

## Decision
- `fields s` : `str -> list str`. Items are the maximal runs of bytes that
  are not ASCII whitespace (` \t\n\r\v\f`), in order; `fields ""` and
  `fields "  "` are `[]`. Non-ASCII bytes are never separators.
- Pure, no trap; both engines identical (T-020).
- `fields` becomes a reserved name (T005); the HumanEval translator's
  reserved list is brought up to date (it lacked the ADR-0042..0046 names).
- llms.txt: `fields str->list str (split on whitespace)`, paid by shortening
  the capability prefix rules in section 8.

## Consequences
- `text-top-words` and the word-counting probe tasks lose their filter.
- Programs that used `fields` as an identifier stop compiling (T005); none
  in the repository does.
