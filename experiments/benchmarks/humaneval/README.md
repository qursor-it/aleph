# HumanEval in Aleph

HumanEval translated into Aleph with the method of [MultiPL-E](https://github.com/nuprl/MultiPL-E): each Python problem becomes an Aleph prompt and a set of hidden tests, by a small type-directed compiler. Because MultiPL-E has published results for many languages on the same problems, Aleph can be compared with them directly.

## What is here

| File | |
|---|---|
| `humaneval_to_aleph.py` | the translator |
| `humaneval-aleph.jsonl` | the translated problems: 156 of 161 |
| `skipped.json` | the 5 problems that cannot be expressed in Aleph, with the reason |
| `evaluate.py` | runs completions against the hidden tests; `--validate` checks the translation itself |
| `test_humaneval_to_aleph.py` | offline tests of the translator |

## How a problem is translated

- **Source**: MultiPL-E's typed HumanEval problems (`datasets/originals-with-cleaned-doctests`), where the MultiPL-E authors added type annotations to every function.
- **Types**: `int` → `i64`, `float` → `f64`, `str`, `bool`, `List[T]` → `list T`, `Optional[T]` → `opt T`, `Dict[K, V]` → `dict K V` (keys `str`, `i64` or `bool`), `Tuple[...]` → a record type `tupN` declared in the prompt. `Any`, `Union` and variable-length tuples are skipped.
- **Names**: Python names lose their underscores (`has_close_elements` → `hascloseelements`); names that clash with Aleph keywords or builtins get an `x` suffix.
- **Prompt**: record types, the function header, a `doc` line with the description, and the docstring examples rewritten as `ex` lines (MultiPL-E's "transform" variant). The model writes the rest: its own `ex` lines, the body, and any helper functions.
- **Tests**: every `assert candidate(...) == value` becomes a hidden `ex` line. A function returning a dict is tested through `keys` and `vals`, since dicts are not comparable. Assertions of any other shape (for example tolerances on floats) are dropped and counted.
- **Stop tokens**: unlike MultiPL-E, a new top-level `fn` does not end the completion, because Aleph has no lambdas and helper functions are part of a solution.

## Evaluation

The program is prompt + hidden tests + completion, formatted with `aleph fmt` so the order of functions is never penalised, then run with `aleph test`. Outcomes: `pass`; `wrong` (compiles, but a test fails or traps); `no-compile`.

```sh
python3 humaneval_to_aleph.py PATH_TO/MultiPL-E/datasets/originals-with-cleaned-doctests humaneval-aleph.jsonl
python3 evaluate.py humaneval-aleph.jsonl --validate           # every translation type-checks with a stub
python3 evaluate.py humaneval-aleph.jsonl completions.jsonl    # {"name", "completion"} per line
```

Current state (compiler at `agent/aleph-m3`, 18 Nov 2026): 156 problems translated, all 1,092 of their equality assertions translated, every translation type-checks, hand-written correct solutions pass and wrong ones fail.

## Licences and a restriction

- HumanEval: © OpenAI, MIT licence.
- The typed problems come from MultiPL-E: © 2022 Northeastern University, Oberlin College, Roblox Inc, Stevens Institute of Technology, University of Massachusetts Amherst and Wellesley College, BSD 3-Clause License **with a Machine Learning Restriction**: its contents may not be used as training data for any machine learning model. `humaneval-aleph.jsonl` is derived from them, so the same restriction applies: it is an **evaluation** set only, and it must never enter the training data of EXP-004.
