# EXP-003 token corpus

The same verified program written in several languages, plus a verifier and a
token-count report for [EXP-003](../../EXP-003-token-corpus.md). Python 3
standard library only, offline.

## Layout

```
experiments/corpus/EXP-003/
  <program>/
    meta.json      {"program", "group", "category", "source"}
    tests.json     shared hidden tests, EXP-002 suite format
    aleph.aph      one file per language
    python.py
    c.c            optional
    bash.sh        optional
    inputs/        optional files copied into the run directory
```

`group` is `bench` or `exp002-samples`. `category` is one of `text`, `files`,
`data`, `json`, `algorithm`. `source` maps each language to the reference it
was copied from. C and bash are optional; the report prints coverage.

The Stage A corpus is seeded from:

- the 15 `bench/tasks.md` programs: `bench/ref/py/*.py`, `bench/ref/c/*.c`,
  and the matching goldens `tests/aph/bench_*.aph` (with `.out`, `.args`,
  `.files`, `.exit` and `tests/aph/expect/*` giving the shared tests);
- the 5 `experiments/tasks/EXP-002-samples/` tasks: `ref/solution.{aph,py,sh}`,
  `tests.json` and `inputs/`. They have no C version.

`meta.json` is not `task.json`: the tools materialise a minimal task directory
for `suite.run_tests`, so the corpus stays independent of the EXP-002 task
metadata (`statement`, `bash_natural`, ...).

## Commands

```sh
# Run every language version against tests.json.
ALEPH=$PWD/target/debug/aleph python3 experiments/tools/corpus/corpus.py \
    verify experiments/corpus/EXP-003

# Token counts and the ratio report (chars4 estimate when no BPE vocabulary).
ALEPH=$PWD/target/debug/aleph python3 experiments/tools/corpus/corpus.py \
    report experiments/corpus/EXP-003

# Same, machine-readable.
ALEPH=$PWD/target/debug/aleph python3 experiments/tools/corpus/corpus.py \
    report experiments/corpus/EXP-003 --json

# Real tokenizers, when T-039 provides them: one --vocab per local file.
... report experiments/corpus/EXP-003 \
    --vocab /path/to/o200k_base.tiktoken \
    --vocab /path/to/DeepSeek-V3.json --seed 0
```

`ALEPH` defaults to `target/debug/aleph` in the repository. `verify` exits 1
if any test fails. The corpus time limit is 120 s because the debug interpreter
needs more than the suite default of 10 s for `bench_fib.aph`.

Tests (offline; the Aleph cases skip without an `ALEPH` binary):

```sh
cd experiments/tools/corpus
ALEPH=/path/to/aleph python3 -m unittest -v test_corpus
```

## Normalisation

Each version is counted twice where the distinction exists:

- **Aleph**: `aleph fmt` output. The "with examples" count keeps `ex` lines;
  the "without examples" count drops them.
- **Python**: `ast.unparse` with module, class and function docstrings removed
  (the same rule as `experiments/pilot/tokens/count.py`). Comments disappear
  with `ast.unparse`.
- **C**: `//` and `/* */` comments stripped with string/char literals left
  intact; `clang-format` is applied when installed.
- **bash**: `#` comments stripped, respecting quotes and `${x#y}`; `shfmt` is
  applied when installed.

The report names the formatter actually used for C and bash (`none` when the
tool is not installed). Python, C and bash have one normalised form, used for
both the with-examples and without-examples ratios; only Aleph differs.

## Tokenizers

Counts come from `aleph tokens FILE --tokenizer all --json` run on the
normalised text in a temporary file. For every `--vocab PATH`, the tool retries
with `--vocab PATH` and merges the resulting `bpe:<stem>` counters. If the
binary rejects `--vocab`, the report records the vocabulary as unavailable and
falls back to the built-in counters.

The analysis uses `bpe:*` counters when any are available, otherwise `chars4`
(`ceil(unicode chars / 4)`) labelled as an estimate. Per-program data always
contains every counter returned by `aleph tokens`.

## Report

For each analysis tokenizer the report prints:

- tokens per program (`aleph` without examples, `aleph` with examples, Python,
  C, bash) and the Aleph/Python ratio;
- the median Aleph/Python ratio with and without examples, a seeded 95%
  percentile bootstrap interval of the median (2000 resamples by default), and
  a two-sided Wilcoxon signed-rank test on the per-program token differences
  (exact for n <= 20, normal approximation with continuity correction above);
- the same statistics by group and by category;
- language coverage;
- break-even `N = tokens(llms.txt) / (tokens(Python) - tokens(Aleph))` per
  group, using pooled without-examples totals; `never` when Aleph is not
  shorter or llms.txt is missing.

`--seed` controls the bootstrap. The same corpus, seed and tokenizer files give
byte-identical JSON.
