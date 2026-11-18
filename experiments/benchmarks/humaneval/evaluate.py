#!/usr/bin/env python3
"""Run completions against the Aleph HumanEval tests.

Usage:
  evaluate.py DATASET.jsonl --validate            check every translation with its stub
  evaluate.py DATASET.jsonl COMPLETIONS.jsonl     score completions

COMPLETIONS.jsonl has one {"name": ..., "completion": ...} per line; the
completion is the text the model wrote after the prompt. The program is
prompt + hidden tests + completion, cut at the first stop token, formatted
with `aleph fmt` (so the order of functions is never penalised) and run with
`aleph test`. A problem passes when `aleph test` exits 0. Results are pass, wrong
(compiles but a test fails or traps) or no-compile.

Needs the `aleph` binary on PATH or in ALEPH. Standard library only.
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

ALEPH = os.environ.get("ALEPH", "aleph")
TIMEOUT = 20


def program(record, completion):
    for stop in record["stop_tokens"]:
        cut = completion.find(stop)
        if cut != -1:
            completion = completion[:cut]
    if not completion.endswith("\n"):
        completion += "\n"
    return record["prompt"] + record["tests"] + completion


def run(source):
    """(status, diagnostics): pass, wrong (compiles, a test fails or traps) or
    no-compile (lexical, parse, format, type or capability errors)."""
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "solution.aph"
        path.write_text(source, encoding="utf-8")
        try:
            fmt = subprocess.run([ALEPH, "fmt", "-w", str(path)], capture_output=True, text=True, timeout=TIMEOUT)
            if fmt.returncode != 0:
                return "no-compile", fmt.stdout + fmt.stderr
            test = subprocess.run([ALEPH, "test", str(path), "--json"], capture_output=True, text=True,
                                  timeout=TIMEOUT, cwd=tmp)
        except subprocess.TimeoutExpired:
            return "wrong", "timeout"
        if test.returncode == 0:
            return "pass", ""
        codes = set()
        for line in (test.stdout + test.stderr).splitlines():
            try:
                codes.add(json.loads(line).get("code", ""))
            except ValueError:
                pass
        status = "wrong" if codes and all(c.startswith(("E00", "R0")) for c in codes) else "no-compile"
        return status, (test.stdout + test.stderr)[:2000]


def main(argv):
    if len(argv) != 2:
        raise SystemExit(__doc__)
    records = [json.loads(l) for l in open(argv[0], encoding="utf-8") if l.strip()]
    if argv[1] == "--validate":
        bad = 0
        for record in records:
            status, output = run(program(record, record["stub"]))
            if status == "no-compile":
                bad += 1
                print(f"{record['name']}: translation does not check\n{output}\n")
        print(f"{len(records)} problems, {bad} with translation errors")
        return 1 if bad else 0
    completions = {}
    for line in open(argv[1], encoding="utf-8"):
        if line.strip():
            item = json.loads(line)
            completions.setdefault(item["name"], []).append(item["completion"])
    results, passed = [], 0
    for record in records:
        for completion in completions.get(record["name"], []):
            status, output = run(program(record, completion))
            passed += status == "pass"
            results.append({"name": record["name"], "status": status, "output": output})
    json.dump(results, sys.stdout, indent=1)
    print(f"\n{passed}/{len(results)} passed", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
