# EXP-002 task suite

Format, validator and runner for the task suite of
[EXP-002](../../EXP-002-ab-python.md). Python 3 standard library only, offline.

## Layout

One folder per task, named after its id:

```
SUITE_DIR/
  <id>/
    task.json
    inputs/            files copied into the run directory before every test
    tests.json         hidden tests
    ref/               optional reference solutions (validator only)
      solution.py
      solution.sh
      solution.aph
```

`inputs/` may be empty or missing when the task only uses arguments. Paths in
`task.json`, test `files` and `expect_files` are relative to the run directory.

## task.json

```json
{
  "id": "files-copy-prefix",
  "category": "files",
  "statement": "Natural-language, language-neutral statement ...",
  "bash_natural": true,
  "files": {"read": ["in.txt"], "write": ["out.txt"]},
  "time_limit_s": 10
}
```

- `id`: string, equal to the folder name.
- `category`: one of `text`, `files`, `data`, `json`, `algorithm`.
- `statement`: non-empty; says what the program reads (arguments, stdin,
  files) and what it prints or writes. It never mentions a language.
- `bash_natural`: whether bash is a natural solution for the task.
- `files.read`: paths the program may read; each must exist in `inputs/` or in
  every test's `files`.
- `files.write`: paths the program may write; every `expect_files` path must
  be listed here.
- `time_limit_s`: positive number, default `10`, applied to each test.

## tests.json

A non-empty list with at least two entries:

```json
[
  {
    "name": "basic",
    "args": ["an"],
    "stdin": "",
    "files": {"in.txt": "one\ntwo\n"},
    "stdout": "an\n",
    "exit_code": 0,
    "expect_files": {"out.txt": "an\n"}
  }
]
```

- `name`: unique inside the task.
- `args`: list of strings passed to the program.
- `stdin` (optional): text fed to stdin. Without it stdin is empty.
- `files` (optional): extra or replacement input files for this test; written
  after `inputs/`, so an entry overrides a file with the same path.
- `stdout`: expected exact stdout.
- `exit_code` (optional): expected exit code, default `0`.
- `expect_files` (optional): files that must exist after the run with exactly
  the given contents.

## Reference solutions

`ref/solution.py`, `ref/solution.sh` and `ref/solution.aph` are used only by
`validate --refs`; the harness never sees them. Write a bash reference only
when `bash_natural` is true. Aleph solutions are run with `ALEPH run`; the
binary comes from the `ALEPH` environment variable, else `aleph` on `PATH`.

## Commands

Run these from the repository root.

```sh
# Structural checks; --refs also runs every reference solution.
python3 experiments/tools/suite/suite.py validate experiments/tasks/EXP-002-samples
ALEPH=$PWD/target/debug/aleph python3 experiments/tools/suite/suite.py validate experiments/tasks/EXP-002-samples --refs

# Run one program against one task.
python3 experiments/tools/suite/suite.py run experiments/tasks/EXP-002-samples/algorithm-primes \
    python experiments/tasks/EXP-002-samples/algorithm-primes/ref/solution.py
python3 experiments/tools/suite/suite.py run ... --json
```

`validate` prints one line per problem as `TASK: message` and exits 1 if there
is any. `run` exits 0 only when all tests pass.

Tests (offline; Aleph cases skip without an `ALEPH` binary):

```sh
cd experiments/tools/suite && python3 -m unittest -v test_suite
```

## Python API

`run_tests(task_dir, lang, program_path, aleph=None)` returns a list of
`{"name", "passed", "detail"}`. `lang` is `python`, `bash` or `aleph`; the
harness T-037 reuses this function. Pass `aleph` explicitly or set `ALEPH`.
