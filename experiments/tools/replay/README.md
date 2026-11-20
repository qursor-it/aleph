# Replay metric

Re-checks every recorded Aleph answer (pilot probe runs, harness logs) with the
current compiler and reports the compile-errors-per-problem metric: check-clean
rate raw and after `aleph fmt`, errors per answer, and diagnostic code counts,
per run file and condition. Re-run it after each language change to see its
effect on real model output.

```sh
python3 replay.py                      # default: experiments/pilot/probe/runs/*.jsonl
python3 replay.py harness-log.jsonl    # plus harness logs (records with `program`, condition `aleph*`)
python3 replay.py --json --out r.json  # sorted-keys JSON; nothing is written unless --out
python3 test_replay.py
```

`ALEPH` selects the binary (default `target/debug/aleph`). Python runs are skipped.

Limits: the pilot sample is tiny (about ten problems per run, runs overlap); only
the compiler is used, no tests are executed, so "clean" is not "correct".
