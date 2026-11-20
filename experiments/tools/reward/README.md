# EXP-004 reward function (T-042)

Pure function from a program and a task folder to a reward in [0, 1], for the
RL arms of [EXP-004](../../EXP-004-finetune-rl.md). Python 3 standard library
only, offline, never calls a model. Task format: [suite](../suite/README.md).

## Definition

`reward = (hidden tests passed / total) * gates`, where a failed gate gives 0.

- Aleph: `aleph check --json` must exit 0 (else `check_ok` false, reward 0),
  and no test may trap with a run-time capability violation (`R003`, exit 70;
  the static codes C001/C002/C003 are caught by the check gate).
- Python: no file access outside the task's declared `files.read`/`files.write`
  (sandbox `outside`, see [sandbox](../sandbox/README.md)); else reward 0 and
  `capability_violation` true.
- Time limit per test: the task's `time_limit_s`, or the `time_limit` argument.
  A timed-out test fails; `timed_out` is true if any did.
- The program's own `ex` lines are only part of `aleph check` (a gate). They
  never add reward. Only the task's hidden tests count.

Result keys: `reward`, `tests_passed`, `tests_total`, `check_ok` (null for
Python), `capability_violation`, `timed_out`, `detail`.

## CLI

```sh
ALEPH=$PWD/target/debug/aleph python3 experiments/tools/reward/reward.py TASK_DIR PROGRAM_FILE [--lang aleph|python] [--time-limit S] [--json]
```

The binary comes from the environment: `ALEPH`, else `target/debug/aleph`.

## API

- `reward(program_text, task_dir, lang="aleph", aleph=None, time_limit=None) -> dict`
- `reward_many(items, workers=1) -> list[dict]`: `items` are dicts of `reward()`
  keyword arguments (`program_text` and `task_dir` required). Errors in one item
  give reward 0 with `detail` "error: ...", never an exception.

Thread safety: `reward()` keeps no shared state (the program and a copy of the
task live in a private temp dir; the task folder is never modified), so it can
be called from several threads or processes. `reward_many` itself is
sequential; `workers` is ignored.

## Limits

- Each call runs the program once per test (plus one re-run of the failed tests
  for Aleph, to read the `R003` trap from stderr). Cost is tests x time limit
  in the worst case.
- Python access logging is a logging sandbox, not isolation; it records, it
  does not block. Network and subprocess use are not checked.
- A trap is detected only on a test that fails; a program that traps `R003`
  on a test that expects that exit code and stdout would not be flagged.
- Infinite loops are bounded by the time limit only (`aleph check` itself has a
  60 s timeout).

Tests: `cd experiments/tools/reward && ALEPH=... python3 -m unittest -v test_reward`.
