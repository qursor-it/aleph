# Orbis generator (T-041)

Turns EXP-004 task statements into verified Aleph programs and repair triples
for the Orbis training corpus. Python 3 standard library only, offline in its
tests; it reuses the [EXP-002 harness](../harness/harness.py) backends and the
[suite runner](../suite/suite.py). See
[EXP-004](../../EXP-004-finetune-rl.md) and its Amendments.

**The agent never runs it with a real model.** Tests use `fake:` answers; only
a person starting a corpus run uses an OpenAI-compatible backend.

## Input

JSONL, one task statement per line:

```json
{"id": "count-lines", "statement": "Read in.txt ...", "category": "files",
 "tests": [{"name": "basic", "args": [], "stdout": "2\n"}],
 "files": {"read": ["in.txt"], "write": []}}
```

`tests` is optional and uses the [suite](../suite/README.md) `tests.json`
format. `files` is optional and uses the suite `files` object. A statement with
tests is kept only if the program also passes every hidden test.

## Commands

Run from the repository root:

```sh
ALEPH=$PWD/target/debug/aleph python3 experiments/tools/orbis/orbis.py generate \
    statements.jsonl \
    --model fake:answers.json \
    --out-dir /tmp/orbis \
    --attempts 3

# Real runs (a person starts these): the cap and prices are mandatory.
python3 experiments/tools/orbis/orbis.py generate statements.jsonl \
    --model openai:MODEL --out-dir DIR \
    --attempts 3 --cap-eur 5.0 --prices prices.json

# Drop statements close to the EXP-002 suite and to HumanEval-Aleph prompts.
python3 experiments/tools/orbis/orbis.py generate statements.jsonl \
    --model fake:answers.json --out-dir DIR \
    --dedup-against experiments/tasks/EXP-002-samples \
                    experiments/benchmarks/humaneval/humaneval-aleph.jsonl
```

Options:

- `--attempts N`: generation attempts per statement (default 3).
- `--cap-eur X`: spending cap, required for every non-fake backend. The worst
  case of each call is checked before the call, exactly as in the harness.
- `--prices P.json`: harness price table, required with `--cap-eur`.
- `--dedup-against PATH...`: EXP-002 suite directories and/or JSONL files with
  a `prompt` or `statement` field. Only that text field is read; hidden tests
  are never opened.
- `--threshold T`: word 5-gram Jaccard similarity at which a statement is
  dropped (default `0.5`).
- `--llms PATH`, `--max-tokens N`, `--seed N`: passed to the harness prompt,
  cap estimate and backend.
- `ALEPH` selects the binary (default `target/debug/aleph`).

## Output

```
DIR/
  programs/<id>.aph   canonical, check-clean, tests-passing programs
  index.jsonl         {id, statement, category, path}, one line per program
  repairs.jsonl       {id, broken, diagnostics, fixed}
  summary.json        kept, dropped, failed, repairs, cost, stopped, details
```

For each attempt the model gets one Aleph prompt with `llms.txt` and the
statement. The answer's last fenced block is checked with
`aleph check --json`. Only diagnostics with `severity: "error"` fail an attempt;
the F001 layout warning is not a failure and its `canonical` text (or
`aleph fmt`) is saved, so every kept program is canonical. Errors and failing
tests are fed back as JSON diagnostics and retries, up to `--attempts`.

A repair triple is written for every failing attempt of a statement that is
later solved, with `broken` the failing answer, `diagnostics` the JSON
diagnostics emitted by that check, and `fixed` the canonical successful
program. Statements that never verify are counted in `failed`.

`summary.json` also has `dropped_details` (id, nearest match and score) and
`failed_details`. Given the same fake answers and inputs, the output is
byte-identical; diagnostics use the relative file name `program.aph`, not a
temporary path.

## Deduplication

Before any model call, statements are normalised to lowercase words and
compared by the Jaccard similarity of their word 5-gram sets. A statement is
dropped when its best score against any `--dedup-against` source is at least
`--threshold`. Dropped ids and their nearest source are logged to stderr and
in `summary.json`; they never reach the model.

## Tests

Offline, fake model only; Aleph-dependent cases skip without the binary:

```sh
cd experiments/tools/orbis
ALEPH=/path/to/aleph python3 -m unittest -v test_orbis
```
