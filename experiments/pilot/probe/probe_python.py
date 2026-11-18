#!/usr/bin/env python3
"""Python baseline of the probe: same problems, same model and settings as
probe_opencode.py, prompts and tests from MultiPL-E's typed HumanEval.

Usage: python3 probe_python.py [--model ...] [--only 1,2] [--out FILE]
Then: python3 probe_python.py --score FILE   (runs each answer against its tests)
"""
import argparse
import json
import pathlib
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from probe import code_block  # noqa: E402
from probe_opencode import run  # noqa: E402


def score(path):
    data = {p["name"]: p for p in json.loads((HERE / "probe-python.json").read_text())["problems"]}
    rows = []
    for line in open(path, encoding="utf-8"):
        item = json.loads(line)
        problem = data[item["name"]]
        program = item["code"] + "\n\n" + problem["test"] + f"\n\ncheck({problem['entry_point']})\n"
        with tempfile.TemporaryDirectory() as tmp:
            file = pathlib.Path(tmp) / "solution.py"
            file.write_text(program, encoding="utf-8")
            try:
                done = subprocess.run([sys.executable, str(file)], capture_output=True, text=True, timeout=20, cwd=tmp)
                status = "pass" if done.returncode == 0 else ("wrong" if "AssertionError" in done.stderr else "no-compile" if "SyntaxError" in done.stderr else "wrong")
            except subprocess.TimeoutExpired:
                status = "wrong"
        rows.append({"name": item["name"], "status": status, "tokens": item["tokens"], "timed_out": item.get("timed_out")})
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="opencode/deepseek-v4.1-flash-free")
    parser.add_argument("--variant", default="high")
    parser.add_argument("--timeout", type=int, default=900)
    parser.add_argument("--only", default="")
    parser.add_argument("--out", default=str(HERE / "probe-python-results.jsonl"))
    parser.add_argument("--score")
    args = parser.parse_args()
    if args.score:
        print(json.dumps(score(args.score), indent=1))
        return
    probe = json.loads((HERE / "probe-python.json").read_text(encoding="utf-8"))
    wanted = {int(n) for n in args.only.split(",") if n}
    with open(args.out, "a", encoding="utf-8") as out:
        for number, problem in enumerate(probe["problems"], 1):
            if wanted and number not in wanted:
                continue
            message = probe["instruction"] + " Do not use tools: answer directly.\n\n```python\n" + problem["prompt"] + "```"
            started = time.time()
            reasoning, answer, tokens, session, rc, err = run(args.model, args.variant, message, args.timeout)
            record = {"name": problem["name"], "language": "python", "model": args.model, "variant": args.variant,
                      "session": session, "date": time.strftime("%Y-%m-%d"), "seconds": round(time.time() - started, 1),
                      "reasoning": reasoning, "answer": answer, "code": code_block(answer), "tokens": tokens,
                      "exit_code": rc, "timed_out": rc == -9}
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            out.flush()
            print(f"{number:02d} {problem['name']}: rc={rc} {len(reasoning)} chars of reasoning, {round(time.time() - started)} s", flush=True)


if __name__ == "__main__":
    main()
