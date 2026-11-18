# Token pilot, 18 Nov 2026: the same verified programs in Aleph and Python

A pilot ahead of EXP-003 (still `draft`), not the experiment itself. No model calls.

## Corpus

- **61 HumanEval problems**: Python is the original reference solution (`canonical_solution`); Aleph is a hand-written solution (`solutions.py`) that passes every hidden test of our translation (`evaluate.py`: 61/61). Picked from the problems Aleph can express today: integers, strings and lists; nothing that needs int↔float conversion or `ord`/`chr`.
- **16 bench programs**: `bench/ref/py/*.py` against the goldens `tests/aph/bench_*.aph` (and `bench/heavy/fib35.aph`).

Normalisation: Python parsed and printed with `ast.unparse` (comments and docstrings removed, type hints kept where the original has them). Aleph canonical (`aleph fmt`), `doc` lines removed, counted without `ex` lines (logic) and with them (`aleph_ex`, only the prompt's visible examples and one per helper; hidden tests excluded).

Tokenizers: `o200k_base`, `cl100k_base`, DeepSeek-V3 and Qwen2.5-Coder (`tokenizer.json` from Hugging Face). Files and HumanEval data are not committed: put `DeepSeek-V3.json`, `Qwen2.5-Coder-7B-Instruct.json` and `HumanEval.jsonl.gz` in a folder and run `VOCAB=folder ALEPH=target/release/aleph python3 experiments/pilot/tokens/count.py`.

## Results (o200k; the other tokenizers within a few points)

| | programs | Aleph/Python median | 95% CI | pooled | Aleph shorter |
|---|---|---|---|---|---|
| Logic only, all | 77 | 1.16 | 1.05–1.29 | 1.24 | 27 |
| Logic only, HumanEval | 61 | 1.23 | 1.05–1.35 | 1.21 | 20 |
| Logic only, bench | 16 | 1.13 | 0.75–1.65 | 1.32 | 7 |
| With `ex` lines, all | 77 | 2.18 | 1.90–2.40 | 2.30 | 2 |

Qwen2.5-Coder is the least favourable (logic median 1.24). `llms.txt` is about 2,900 tokens.

## Where the tokens go

- Function bodies alone are shorter than Python's: median ratio **0.74**.
- Signatures are **39%** of Aleph's tokens. Aleph programs have 1.44 helper functions on average, Python 0.13: with no lambdas or closures, every `map`/`filter`/`fold` callback and every loop becomes a named, fully typed function, often with an accumulator parameter.
- Missing builtins add helpers: `abs`, `min`/`max`, `zip`, set, string prefix/slicing, `enumerate`.
- Aleph wins where the Python reference loops by hand (`will_it_fly` 0.37, `is_palindrome` 0.43) and loses where Python has a builtin or a comprehension (`unique` 3.3, `compare` 4.2, `bench/maxmin` 3.4).

## Reading

H3.1 (median ratio ≤ 0.8) is far from holding today. The cost is not the prefix notation, which makes bodies shorter, but the language's missing pieces: anonymous functions, a few builtins, and mandatory examples. Candidates to test, one at a time, by re-counting this same corpus: inline lambdas for higher-order builtins; `abs min max zip` and string `slice`/`starts`; counting `ex` as tests rather than source.

Threats: one author for the Aleph versions (me), HumanEval references are not always idiomatic Python, small programs exaggerate signature cost.
