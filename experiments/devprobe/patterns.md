# Where model answers spend more than the reference (22 Nov 2026)

Input: the nine recorded runs (run1-run9, 180 answers, 281 functions) and
the four reference files (80 tasks, 96 functions), all in this directory.
Counts are regex counts over the answer text; sizes are characters of the
fenced code. No model was called for this note.

## Size
Median answer/reference size ratio per task: 1.29 (180 pairs). Per set:
set 1 (run1, run4) 5813/5204 vs 4764; set 2 (run2, 3, 5, 7) 4936 down to
3520 vs 2913; set 3 (run6) 4891 vs 3461; set 4 (run8, run9) 9500/8082 vs
5396. The gap grows with task difficulty: on set 4 the answers are 1.5-1.8x
the reference, with 43/39 functions against 31.

## Patterns (runs vs references)

| pattern | runs (180 answers) | refs (80) |
|---|---|---|
| empty-list test `== len X 0` | 61 | 14 |
| head `else at X 0 D` | 53 | 18 |
| tail `slice X 1 len X` | 37 | 8 |
| index-loop parameter `i:i64` | 16 | 0 |
| test then unwrap (`if not isok`/`if not has`, then `else X d`) | 22 | 2 |
| `okor` | 5 | 7 |
| `fold` / `map` / `filter` / `mapr` | 35 / 43 / 21 / 6 | 25 / 27 / 19 / 4 |

1. **Hand-written head/tail recursion** is the main cost: a helper with an
   accumulator, `if == len xs 0 BASE`, `let x else at xs 0 D`, a recursive
   call on `slice xs 1 len xs`, where the reference uses `fold`, `map` or
   `mapr` (run9 `topn` builds counts with a recursive `tally`; run9
   `jsonsum` sums with a recursive `sumint` instead of `mapr`; run3
   `cumsum`). Each such helper costs a header, an `ex` line and 3-4 body
   lines.
2. **Test then unwrap survives `okor`.** In set 4 (run8, run9, after
   ADR-0048) there are 11 `if not isok X err E` / `let v else X d` pairs and
   0 uses of `okor`; run7 (set 2) used `okor` 5 times. `okor` is only in the
   builtin table; no example shows it. Each pair is about 45 characters
   where `let v try okor X E` is about 26.
3. **A tail builtin would save little.** `slice X 1 len X` -> a `rest X`
   would save about 10 characters per use, about 1% of answer text, for a
   new reserved name and an llms.txt line. Not proposed.

## Candidate changes (not made)
- **Show `okor` in an llms.txt example** (about 15 characters of the ~57
  tokens freed by T-074), e.g. `run_idioms` `total` as
  `try mapr lam t okor int t t ...`; then re-run set 4 and count `okor` uses
  and test-then-unwrap pairs. Cheapest test of "a builtin in the table only
  is not found".
- **One line nudging higher-order builtins over recursion** (about 10
  tokens): measure helper count and answer size on set 4 before and after.
  Risk: a nudge may push models into lambdas they get wrong; the probe
  would show it as compile errors.
Both are llms.txt text changes, so each needs a before/after probe run, and
run-to-run noise is about 2 tasks per set.
