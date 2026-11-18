#!/usr/bin/env python3
"""Compare probe conditions: pass rate and median tokens on the problems that
every condition answered.

Usage: ALEPH=path/to/aleph python3 compare.py python=PY.jsonl full=AL.jsonl lite=LITE.jsonl ...
A condition named `python` is scored with HumanEval's Python tests, every other
one with the Aleph hidden tests.
"""
import json
import statistics
import sys
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "benchmarks" / "humaneval"))
import analyze  # noqa: E402
import evaluate  # noqa: E402
from probe_python import score  # noqa: E402


def load(path):
    out = {}
    for line in open(path, encoding="utf-8"):
        item = json.loads(line)
        if item.get("answer") and not item.get("timed_out"):
            out[item["name"]] = item
    return out


def aleph_status(items):
    dataset = {json.loads(l)["name"]: json.loads(l) for l in open(analyze.DATASET, encoding="utf-8")}
    result = {}
    for name, item in items.items():
        source = analyze.with_tests(item["code"], dataset[name])
        result[name] = "no-compile" if source is None else evaluate.run(source)[0]
    return result


def main(argv):
    conditions = dict(arg.split("=", 1) for arg in argv)
    answers, status = {}, {}
    for label, path in conditions.items():
        answers[label] = load(path)
        if label == "python":
            tmp = pathlib.Path("/tmp/compare-python.jsonl")
            tmp.write_text("".join(json.dumps(i) + "\n" for i in answers[label].values()))
            status[label] = {r["name"]: r["status"] for r in score(tmp)}
        else:
            status[label] = aleph_status(answers[label])
    common = sorted(set.intersection(*(set(a) for a in answers.values())), key=lambda n: int(n.split("_")[1]))
    print(f"problems answered in every condition: {len(common)}")
    print("condition | pass | input (median) | reasoning (median) | reasoning (total) | output (median) | total (median)")
    for label in conditions:
        tokens = [answers[label][n]["tokens"] for n in common]
        passed = sum(status[label][n] == "pass" for n in common)
        total = [t["input"] + t["reasoning"] + t["output"] for t in tokens]
        med = lambda key: int(statistics.median(t[key] for t in tokens))
        print(f"{label} | {passed}/{len(common)} = {100 * passed / len(common):.1f}% | {med('input')} | {med('reasoning')} | "
              f"{sum(t['reasoning'] for t in tokens)} | {med('output')} | {int(statistics.median(total))}")
    print("\nper problem (status, reasoning tokens):")
    for n in common:
        print(n, " | ".join(f"{label}: {status[label][n]} {answers[label][n]['tokens']['reasoning']}" for label in conditions))


if __name__ == "__main__":
    main(sys.argv[1:])
