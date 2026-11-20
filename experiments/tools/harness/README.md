# EXP-002 A/B harness (T-037)

Runs **one condition on one task** for [EXP-002](../../EXP-002-ab-python.md):
generate a program, check it, run the task's hidden tests, feed back the result,
retry up to `--rounds`. Python 3 standard library only and offline by default.

The harness never calls a real model in its tests. The only network backend is
`openai:`, which is implemented but not executed by the test suite.

## Usage

Run from anywhere (paths are explicit):

```sh
python3 experiments/tools/harness/harness.py run \
  --task experiments/tasks/EXP-002-samples/algorithm-primes \
  --condition python \
  --model fake:answers.json \
  --cap-eur 0.50 \
  --prices prices.json \
  --out run.jsonl \
  --rounds 5 \
  --seed 7

python3 experiments/tools/harness/harness.py summary run.jsonl
```

`--condition` is one of `python`, `bash`, `aleph-llms`,
`aleph-llms-examples`. `--cap-eur`, `--prices` and `--out` are required.

Options:

- `--rounds N`: maximum generation rounds (default 5).
- `--seed N`: passed to the backend (and to OpenAI-compatible APIs).
- `--examples DIR`: task suite used for the five solved examples of
  `aleph-llms-examples`; required for that condition.
- `--llms PATH`: language reference for the Aleph conditions (default
  `<repo>/llms.txt`).
- `--max-tokens N`: `max_tokens` for the model call and for the cap estimate
  (default 4096).
- `ALEPH`: Aleph binary for `aleph check --json` and hidden tests. Default
  `<repo>/target/debug/aleph`.

### Backends

- `fake:<answers.json>`: replays a JSON list. Each entry is an answer string,
  or `{"text": "...", "usage": {"input": N, "cached_input": N, "output": N,
  "reasoning": N}}`. Used by the offline tests.
- `openai:<model>`: OpenAI-compatible `POST /chat/completions` via `urllib`.
  Base URL from `OPENAI_BASE_URL`, key from `OPENAI_API_KEY`; the key is never
  logged or written to the log.

### Prices

`--prices` is:

```json
{"MODEL": {"input": 1.0, "cached_input": 0.5, "output": 2.0}}
```

Values are euro per million tokens. Lookup tries the full `--model` string,
then the `NAME` after `BACKEND:`, then `"default"`. New input is
`input - cached_input`, and reasoning tokens are charged at the output price.
Costs are kept as floats; the log keeps full precision.

## Spending cap

`--cap-eur` is mandatory: without it the command exits 2 with a clear message.
The cap is shared by every record already in `--out` (the sum of their
`cost_eur`), so several invocations can continue the same budget.

Before **every** model call the harness computes the worst-case cost:

```
prompt_chars / 4 * input_price + max_tokens * output_price   (per million)
```

If `previous_spend + worst_case > cap`, no call is made. The log receives an
event `{"event": "stopped", "reason": "cap", "message": "stopped: cap", ...}`
and the run closes with a summary line saying `passed: false`.

## Prompts

The published templates are in [`prompts/`](prompts/), one system and one user
file per condition (`<condition>.system.txt`, `<condition>.user.txt`).
Placeholders: `{statement}`, `{read_files}`, `{write_files}`, `{llms}` and
`{examples}`. Aleph conditions embed the full `llms.txt`; the examples
condition also embeds up to five solved task folders from `--examples`
(`task.json` statement plus `ref/solution.aph`), always excluding the task being
run.

The model must answer with one fenced code block; the harness extracts the
**last** fenced block (falling back to the whole answer if there is none).

## Round loop and feedback

Each round: build the prompt, call the model, static-check the extracted
program, run the hidden tests with `suite.run_tests`, then stop if every test
passed. Otherwise the next round includes feedback with the same shape for
every language:

1. `Static check: ok|failed`.
2. The checker output, truncated to 2000 characters. Checkers:
   - Aleph: `ALEPH check --json FILE`;
   - Python: `python3 -m py_compile FILE`;
   - bash: `bash -n FILE`.
3. The failing hidden tests, one block each: test name, expected stdout, and
   the actual result (stdout, exit code or other `suite` detail), each
   truncated to 500 characters.

Test results are given as feedback exactly as the protocol says, identically
for every language.

## Log schema

One JSON object per line. A round record:

```json
{
  "event": "round",
  "run_id": "0f3c...",
  "task": "algorithm-primes",
  "condition": "python",
  "model": "fake:/tmp/answers.json",
  "seed": 7,
  "round": 1,
  "prompt_chars": 1234,
  "usage": {"new_input": 900, "cached_input": 100, "output": 50, "reasoning": 10},
  "cost_eur": 0.00107,
  "wall_s": 0.42,
  "check_ok": true,
  "tests_passed": 4,
  "tests_total": 4,
  "passed": true,
  "program": "...the extracted program..."
}
```

Then a final line per run:

```json
{
  "event": "summary",
  "run_id": "0f3c...",
  "task": "algorithm-primes",
  "condition": "python",
  "model": "fake:/tmp/answers.json",
  "seed": 7,
  "passed": true,
  "rounds_to_pass": 1,
  "rounds": 1,
  "max_rounds": 5,
  "total_tokens": {"new_input": 900, "cached_input": 100, "output": 50, "reasoning": 10},
  "total_cost_eur": 0.00107
}
```

Cap-stopped runs add the `stopped` record before their summary.

`summary LOG.jsonl` prints JSON grouped by condition with `runs`, `solved`,
`pass_at_1`, `pass_at_k` (any round), `k`, `mean_rounds_to_pass`,
`tokens_per_solved` (by kind), `cost_eur_per_solved`, `total_tokens` and
`total_cost_eur`.

## Tests

Offline; the fake model is the only backend used. The Aleph case is skipped
when no binary is present.

```sh
cd experiments/tools/harness && \
  ALEPH=/home/claude_mobile/projects/aleph/target/debug/aleph \
  python3 -m unittest -v test_harness
```
