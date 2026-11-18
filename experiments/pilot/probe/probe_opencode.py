#!/usr/bin/env python3
"""Run the probe problems through opencode, the same CLI the agent loop uses.

Usage: python3 probe_opencode.py [--model opencode/deepseek-v4.1-flash-free] [--variant high] [--out FILE]

Each problem runs in a fresh, empty directory (no AGENTS.md, no repository),
with `opencode run --thinking --format json`. Reasoning, answer, code block
and token counts are taken from opencode's JSON events and appended to the
output file in the format analyze.py reads. Standard library only.
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


def run(model, variant, message, timeout):
    with tempfile.TemporaryDirectory() as workdir:
        # No tools: in a non-interactive run a tool asking for permission would hang.
        config = {"$schema": "https://opencode.ai/config.json",
                  "permission": {"edit": "deny", "bash": "deny", "webfetch": "deny", "read": "deny",
                                 "external_directory": "deny"}}
        pathlib.Path(workdir, "opencode.json").write_text(json.dumps(config), encoding="utf-8")
        command = ["opencode", "run", "-m", model, "--thinking", "--format", "json", message]
        if variant:
            command[4:4] = ["--variant", variant]
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd=workdir)
        timed_out = False
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            # Keep whatever reasoning arrived before the limit.
            process.kill()
            stdout, stderr = process.communicate()
            timed_out = True
    done = subprocess.CompletedProcess(command, -9 if timed_out else process.returncode, stdout, stderr)
    reasoning, text, tokens, session = [], [], {"input": 0, "cached_input": 0, "output": 0, "reasoning": 0}, None
    for line in done.stdout.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        session = session or event.get("sessionID")
        part = event.get("part", {})
        if event.get("type") == "reasoning":
            reasoning.append(part.get("text", ""))
        elif event.get("type") == "text":
            text.append(part.get("text", ""))
        elif event.get("type") == "step_finish":
            used = part.get("tokens", {})
            tokens["input"] += used.get("input", 0)
            tokens["cached_input"] += used.get("cache", {}).get("read", 0)
            tokens["output"] += used.get("output", 0)
            tokens["reasoning"] += used.get("reasoning", 0)
    return "\n".join(reasoning), "\n".join(text), tokens, session, done.returncode, done.stderr[-500:]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="opencode/deepseek-v4.1-flash-free")
    parser.add_argument("--variant", default="high")
    parser.add_argument("--timeout", type=int, default=900)
    parser.add_argument("--only", default="", help="comma-separated problem numbers, e.g. 3,4,5")
    parser.add_argument("--system", help="file to use instead of llms.txt (e.g. prompts/llms-lite.txt)")
    parser.add_argument("--condition", default="llms", help="label stored with each answer")
    parser.add_argument("--out", default=str(HERE / "probe-results.jsonl"))
    args = parser.parse_args()
    probe = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))
    if args.system:
        probe["system"] = pathlib.Path(args.system).read_text(encoding="utf-8")
        probe["instruction"] += " The order of functions does not matter."
    with open(args.out, "a", encoding="utf-8") as out:
        wanted = {int(n) for n in args.only.split(",") if n}
        for number, problem in enumerate(probe["problems"], 1):
            if wanted and number not in wanted:
                continue
            message = (probe["system"] + "\n\n---\n\n" + probe["instruction"]
                       + " Do not use tools: answer directly.\n\n```aleph\n" + problem["prompt"] + "```")
            started = time.time()
            reasoning, answer, tokens, session, rc, err = run(args.model, args.variant, message, args.timeout)
            record = {"name": problem["name"], "condition": args.condition, "model": args.model, "variant": args.variant, "server": "opencode",
                      "session": session, "date": time.strftime("%Y-%m-%d"), "seconds": round(time.time() - started, 1),
                      "reasoning": reasoning, "answer": answer, "code": code_block(answer), "tokens": tokens,
                      "exit_code": rc}
            if rc != 0:
                record["stderr"] = err
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            out.flush()
            print(f"{number:02d} {problem['name']}: rc={rc} {len(reasoning)} chars of reasoning, {round(time.time() - started)} s", flush=True)


if __name__ == "__main__":
    main()
