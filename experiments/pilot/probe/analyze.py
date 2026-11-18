#!/usr/bin/env python3
"""Score probe answers and look for translation in the reasoning.

Usage: ALEPH=path/to/aleph python3 analyze.py probe-results.jsonl

For each answer: the hidden HumanEval-Aleph tests are inserted into the
model's program right after the target function's header and doc line, the
program is formatted and tested (pass / wrong / no-compile, as in
experiments/benchmarks/humaneval/evaluate.py). The reasoning is scanned for
code in another language: fenced blocks marked python or similar, and lines
that look like Python statements. That is a hint for the `translated`
judgement, which stays a person's call after reading the reasoning.
"""
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "benchmarks" / "humaneval"))
import evaluate  # noqa: E402

DATASET = HERE.parent.parent / "benchmarks" / "humaneval" / "humaneval-aleph.jsonl"
OTHER_FENCE = re.compile(r"```\s*(python|py|javascript|js|c|cpp|java|rust|bash|sh)\b", re.I)
PY_LINE = re.compile(r"^\s*(def \w+\(.*\)\s*:|for \w+ in .+:|while .+:|if .+:\s*$|return [^\n]*|elif .+:|else:\s*$|import \w+|\w+\s*=\s*\[|\w+ \+= )")
ALEPH_LINE = re.compile(r"^\s*(fn \w+ |ex \w+|let \w+ |do |use |type \w+ )")


def with_tests(code, record):
    lines = code.rstrip("\n").splitlines()
    header = next((i for i, l in enumerate(lines) if l.startswith(f"fn {record['entry_point']} ")), None)
    if header is None:
        return None
    at = header + 1
    if at < len(lines) and lines[at].startswith(" doc "):
        at += 1
    tests = record["tests"].rstrip("\n").splitlines()
    return "\n".join(lines[:at] + tests + lines[at:]) + "\n"


def translation_signals(reasoning):
    fences = len(OTHER_FENCE.findall(reasoning))
    py = sum(1 for l in reasoning.splitlines() if PY_LINE.match(l) and not ALEPH_LINE.match(l))
    aleph = sum(1 for l in reasoning.splitlines() if ALEPH_LINE.match(l))
    mentions = len(re.findall(r"\bpython\b", reasoning, re.I))
    return {"other_fences": fences, "python_like_lines": py, "aleph_lines": aleph, "python_mentions": mentions}


def main(argv):
    if len(argv) != 1:
        raise SystemExit(__doc__)
    dataset = {json.loads(l)["name"]: json.loads(l) for l in open(DATASET, encoding="utf-8") if l.strip()}
    rows = []
    for line in open(argv[0], encoding="utf-8"):
        if not line.strip():
            continue
        item = json.loads(line)
        record = dataset[item["name"]]
        source = with_tests(item["code"], record)
        status, output = ("no-compile", "target function missing") if source is None else evaluate.run(source)
        signals = translation_signals(item.get("reasoning", ""))
        rows.append({"name": item["name"], "model": item.get("model"), "status": status,
                     "reasoning_chars": len(item.get("reasoning", "")), **signals,
                     "tokens": item.get("tokens"), "diagnostics": output[:600]})
    print("problem | status | reasoning chars | python-like lines | aleph lines | other-language blocks | 'python' mentions")
    for r in rows:
        print(f"{r['name']} | {r['status']} | {r['reasoning_chars']} | {r['python_like_lines']} | {r['aleph_lines']} | {r['other_fences']} | {r['python_mentions']}")
    passed = sum(r["status"] == "pass" for r in rows)
    print(f"\n{passed}/{len(rows)} pass; {sum(r['other_fences'] > 0 or r['python_like_lines'] >= 3 for r in rows)} answers with signs of drafting in another language")
    out = pathlib.Path(argv[0]).with_suffix(".analysis.json")
    out.write_text(json.dumps(rows, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"details in {out}")


if __name__ == "__main__":
    main(sys.argv[1:])
