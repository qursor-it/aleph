#!/usr/bin/env python3
"""EXP-002 on HumanEval: the same model writes Aleph (with llms.txt) or Python.

Usage:
  humaneval_ab.py run --model BACKEND:NAME --lang aleph|python --out LOG.jsonl
      [--rep N] [--rounds 5] [--only NAME,NAME] [--limit N]
  humaneval_ab.py calibrate --model opencode:NAME     tokens of an empty call
  humaneval_ab.py summary LOG.jsonl [--overhead N]

One session per problem: the model answers, the visible check runs (Aleph:
`aleph check --json`; Python: compile and the prompt's doctests), its output
is fed back, up to --rounds. The session stops at the first answer that
passes the visible check. Hidden tests only score each round; the model never
sees them. Problems: the 156 of humaneval-aleph.jsonl, Python from the
original HumanEval (python/HumanEval.jsonl.gz) for the same task ids.

Backends: opencode:<provider/model> runs `opencode run` with a tool-less agent
(no API key in this process); fake:<answers.json> replays answers (tests).
Python 3 standard library only.
"""
import argparse
import gzip
import hashlib
import json
import os
import pathlib
import random
import re
import statistics
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[2]
HE = REPO / "experiments" / "benchmarks" / "humaneval"
sys.path.insert(0, str(HE))
import evaluate  # noqa: E402

LLMS = REPO / "llms.txt"
ALEPH_BIN = os.environ.get("ALEPH") or str(REPO / "target" / "debug" / "aleph")
evaluate.ALEPH = ALEPH_BIN

FIRST = {
    "aleph": (
        "You are writing code in Aleph. Its complete reference is between the lines.\n"
        "-----\n{llms}-----\n"
        "Complete the function below. Keep its header, doc and ex lines; add the body and any "
        "helper functions (each with at least one ex line).\n"
        "Answer with the complete code in one ```aleph block and nothing else.\n\n{prompt}"
    ),
    "python": (
        "Complete the Python function below. Keep its signature and docstring; add the body "
        "and any imports or helper functions.\n"
        "Answer with the complete code in one ```python block and nothing else.\n\n{prompt}"
    ),
}
FEEDBACK = ("Your code does not pass the checks:\n{output}\n"
            "Fix it. Answer with the complete corrected code in one ```{lang} block and nothing else.")

PLAIN_AGENT = {
    "agent": {
        "plain": {
            "description": "plain completion, no tools",
            "mode": "primary",
            "prompt": "You are a programmer. Answer with code only, as asked.",
            "permission": {"edit": "deny", "bash": "deny", "webfetch": "deny"},
            "tools": {t: False for t in ["bash", "edit", "write", "read", "glob", "grep", "list",
                                         "patch", "todowrite", "todoread", "webfetch", "task"]},
        }
    }
}

PY_CHECK = r'''
import ast, re, sys
ns = {}
src = open("solution.py").read()
exec(compile(src, "solution.py", "exec"), ns)
entry = open("entry.txt").read()
doc = ""
for node in ast.walk(ast.parse(open("prompt.txt").read())):
    if isinstance(node, ast.FunctionDef) and node.name == entry:
        doc = ast.get_docstring(node) or ""
# Same examples as the Aleph ex lines: `>>> entry(...)` and the next line (humaneval_to_aleph.py).
lines, failed = doc.splitlines(), 0
for i, line in enumerate(lines):
    m = re.match(r"^\s*>>>\s*(.+)$", line)
    if not m or i + 1 >= len(lines):
        continue
    call, want = m.group(1).strip(), lines[i + 1].strip()
    try:
        node = ast.parse(call, mode="eval").body
        ast.parse(want, mode="eval")
    except SyntaxError:
        continue
    if not (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == entry):
        continue
    try:
        got = eval(call, ns)
        ok = got == eval(want, ns)
    except Exception as e:
        got, ok = f"{type(e).__name__}: {e}", False
    if not ok:
        failed += 1
        print(f"example failed: {call}  expected {want}  found {got!r}")
sys.exit(1 if failed else 0)
'''


PY_DATA = HE / "python" / "humaneval-python.jsonl"


def build_python(mple_dir):
    """Python prompts of MultiPL-E (the ones the Aleph prompts were translated from) with the
    original HumanEval tests, for the 156 Aleph problems."""
    tests = {r["task_id"]: r for r in map(json.loads, gzip.open(HE / "python" / "HumanEval.jsonl.gz", "rt"))}
    with PY_DATA.open("w") as out:
        for line in open(HE / "humaneval-aleph.jsonl", encoding="utf-8"):
            tid = json.loads(line)["original"]
            number = tid.split("/")[1]
            (path,) = pathlib.Path(mple_dir).glob(f"HumanEval_{number}_*.py")
            prompt = path.read_text(encoding="utf-8").split("### Canonical solution below ###")[0]
            out.write(json.dumps({"task_id": tid, "prompt": prompt.rstrip() + "\n",
                                  "entry_point": tests[tid]["entry_point"], "test": tests[tid]["test"]}) + "\n")


def problems():
    py = {r["task_id"]: r for r in map(json.loads, PY_DATA.open())}
    out = []
    for line in open(HE / "humaneval-aleph.jsonl", encoding="utf-8"):
        al = json.loads(line)
        out.append({"name": al["name"], "task_id": al["original"], "aleph": al, "python": py[al["original"]]})
    return out


def code_block(text, lang):
    blocks = re.findall(r"```[ \t]*(\w*)[^\n]*\n(.*?)```", text, re.S)
    for tag, body in blocks:
        if tag.lower() == lang:
            return body
    return blocks[0][1] if blocks else text


def run_cmd(args, cwd, timeout=20):
    # opencode finds its config through $PWD, which subprocess does not update with cwd
    env = {**os.environ, "PWD": str(cwd)}
    try:
        p = subprocess.run(args, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True,
                           timeout=timeout)
        return p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired:
        return 124, "timeout"


def visible_check(lang, code, prob):
    with tempfile.TemporaryDirectory() as tmp:
        if lang == "aleph":
            pathlib.Path(tmp, "solution.aph").write_text(code, encoding="utf-8")
            rc, out = run_cmd([ALEPH_BIN, "check", "solution.aph", "--json"], tmp)
        else:
            pathlib.Path(tmp, "solution.py").write_text(code, encoding="utf-8")
            pathlib.Path(tmp, "prompt.txt").write_text(prob["python"]["prompt"], encoding="utf-8")
            pathlib.Path(tmp, "entry.txt").write_text(prob["python"]["entry_point"], encoding="utf-8")
            pathlib.Path(tmp, "check.py").write_text(PY_CHECK, encoding="utf-8")
            rc, out = run_cmd([sys.executable, "-I", "check.py"], tmp, timeout=10)
    return rc == 0, out[:3000]


def with_hidden_aleph(code, record):
    """The model's code with the hidden ex lines inserted after the entry fn header and doc."""
    lines = code.rstrip("\n").split("\n")
    entry = record["entry_point"]
    for i, line in enumerate(lines):
        if line == f"fn {entry}" or line.startswith(f"fn {entry} "):
            j = i + 1
            while j < len(lines) and lines[j].startswith(" doc "):
                j += 1
            hidden = record["tests"].rstrip("\n").split("\n")
            return "\n".join(lines[:j] + hidden + lines[j:]) + "\n"
    return None


def hidden_score(lang, code, prob):
    """pass, wrong or no-compile on the hidden tests."""
    if lang == "aleph":
        source = with_hidden_aleph(code, prob["aleph"])
        return "no-compile" if source is None else evaluate.run(source)[0]
    py = prob["python"]
    with tempfile.TemporaryDirectory() as tmp:
        src = f"{code}\n\n{py['test']}\n\ncheck({py['entry_point']})\n"
        pathlib.Path(tmp, "t.py").write_text(src, encoding="utf-8")
        try:
            compile(code, "solution.py", "exec")
        except SyntaxError:
            return "no-compile"
        rc, _ = run_cmd([sys.executable, "-I", "t.py"], tmp, timeout=10)
        return "pass" if rc == 0 else "wrong"


class FakeBackend:
    def __init__(self, path):
        self.answers = json.load(open(path))

    def start(self):
        pass

    def send(self, message):
        return self.answers.pop(0), {}


class BackendError(Exception):
    """The model service failed; the problem is not recorded so a resumed run retries it."""


class OpencodeBackend:
    def __init__(self, model, workdir):
        self.model, self.workdir, self.session = model, pathlib.Path(workdir), None
        self.workdir.mkdir(parents=True, exist_ok=True)
        (self.workdir / "opencode.json").write_text(json.dumps(PLAIN_AGENT, indent=1))

    def start(self):
        self.session = None

    def send(self, message):
        """One model turn; server or rate-limit errors are retried after 30, 60, 120, 240 s."""
        prior = self.session
        for retries, wait in enumerate((30, 60, 120, 240, None)):
            text, usage = self._send_once(message)
            if text:
                usage["retries"] = retries
                return text, usage
            self.session = prior  # a failed call must not become the conversation
            if wait is None:
                raise BackendError(usage.get("error", "empty answer"))
            time.sleep(wait)

    def _send_once(self, message):
        args = ["opencode", "run", "--pure", "--agent", "plain", "-m", self.model, "--format", "json"]
        if self.session:
            args += ["-s", self.session]
        rc, out = run_cmd(args + [message], self.workdir, timeout=900)
        text, usage = [], {"input": 0, "output": 0, "reasoning": 0, "cache_read": 0, "cache_write": 0, "steps": 0,
                       "cost": 0.0}
        for line in out.splitlines():
            try:
                event = json.loads(line)
            except ValueError:
                continue
            self.session = event.get("sessionID") or self.session
            part = event.get("part", {})
            if event.get("type") == "text":
                text.append(part.get("text", ""))
            elif event.get("type") == "step_finish":
                tok = part.get("tokens", {})
                usage["steps"] += 1
                for k in ("input", "output", "reasoning"):
                    usage[k] += tok.get(k, 0)
                usage["cache_read"] += tok.get("cache", {}).get("read", 0)
                usage["cache_write"] += tok.get("cache", {}).get("write", 0)
                usage["cost"] += part.get("cost", 0)  # as reported by opencode (flat-fee plan: list price)
        if rc != 0 and not text:
            usage["error"] = out[-500:]
        return "".join(text), usage


def make_backend(spec, workdir):
    kind, _, name = spec.partition(":")
    if kind == "fake":
        return FakeBackend(name)
    if kind == "opencode":
        return OpencodeBackend(name, workdir)
    raise SystemExit(f"unknown backend {kind!r}")


def git_head():
    rc, out = run_cmd(["git", "rev-parse", "--short", "HEAD"], REPO)
    return out.strip() if rc == 0 else "unknown"


def session(backend, lang, prob, rounds):
    llms = LLMS.read_text(encoding="utf-8")
    message = FIRST[lang].format(llms=llms, prompt=prob[lang]["prompt"])
    backend.start()
    log = []
    for n in range(1, rounds + 1):
        t0 = time.time()
        text, usage = backend.send(message)
        code = code_block(text, lang)
        ok, out = visible_check(lang, code, prob)
        log.append({"round": n, "usage": usage, "wall_s": round(time.time() - t0, 1), "visible_ok": ok,
                    "hidden": hidden_score(lang, code, prob), "code": code, "check": "" if ok else out})
        if ok:
            break
        message = FEEDBACK.format(output=out, lang=lang)
    return log


def cmd_run(a):
    out = pathlib.Path(a.out)
    done = set()
    if out.exists():
        done = {(r["name"], r["lang"], r["rep"]) for r in map(json.loads, out.open())}
    backend = make_backend(a.model, REPO / ".agent-tmp" / "exp002-oc")
    probs = problems()
    if a.only:
        probs = [p for p in probs if p["name"] in a.only.split(",")]
    if a.limit:
        probs = probs[: a.limit]
    meta = {"commit": git_head(), "llms_sha256": hashlib.sha256(LLMS.read_bytes()).hexdigest()[:16]}
    for prob in probs:
        if (prob["name"], a.lang, a.rep) in done:
            continue
        rounds = session(backend, a.lang, prob, a.rounds)
        rec = {"exp": "EXP-002", "bench": "humaneval", "name": prob["name"], "task_id": prob["task_id"],
               "lang": a.lang, "model": a.model, "rep": a.rep, **meta, "rounds": rounds,
               "first": rounds[0]["hidden"], "final": rounds[-1]["hidden"]}
        with out.open("a") as f:
            f.write(json.dumps(rec) + "\n")
        print(f"{prob['name']:28} {a.lang:6} rounds {len(rounds)} first {rec['first']:10} final {rec['final']}",
              flush=True)


def cmd_calibrate(a):
    backend = make_backend(a.model, REPO / ".agent-tmp" / "exp002-oc")
    backend.start()
    print(json.dumps(backend.send("Reply with only the number 4.")[1]))


def summarize(records, overhead=0):
    rows = {}
    for r in records:
        s = rows.setdefault(r["lang"], {"n": 0, "first": 0, "final": 0, "rounds": 0, "new_in": 0, "out": 0,
                                        "calls": 0, "chars": 0, "reasoning": 0})
        s["n"] += 1
        s["first"] += r["first"] == "pass"
        s["final"] += r["final"] == "pass"
        s["rounds"] += len(r["rounds"])
        s["chars"] += len(r["rounds"][-1]["code"])
        for rd in r["rounds"]:
            u = rd["usage"]
            s["calls"] += u.get("steps", 1)
            s["new_in"] += u.get("input", 0) + u.get("cache_read", 0) + u.get("cache_write", 0)
            s["out"] += u.get("output", 0) + u.get("reasoning", 0)
            s["reasoning"] += u.get("reasoning", 0)
    lines = []
    for lang, s in sorted(rows.items()):
        net_in = s["new_in"] - overhead * s["calls"]
        solved = max(s["final"], 1)
        lines.append(f"{lang:6} n {s['n']} pass@1 {s['first'] / s['n']:.3f} final {s['final'] / s['n']:.3f} "
                     f"rounds {s['rounds'] / s['n']:.2f} input-net {net_in} output {s['out']} (reasoning {s['reasoning']}) "
                     f"tokens/solved {(net_in + s['out']) / solved:.0f} out/solved {s['out'] / solved:.0f} "
                     f"final-code chars {s['chars'] / s['n']:.0f}")
    return lines


def session_tokens(rec, overhead):
    """Net input (system prompt removed, spec kept), output and reasoning of one session."""
    net = out = reas = 0
    for rd in rec["rounds"]:
        u = rd["usage"]
        net += u.get("input", 0) + u.get("cache_read", 0) + u.get("cache_write", 0) - overhead * u.get("steps", 1)
        out += u.get("output", 0)
        reas += u.get("reasoning", 0)
    return net, out, reas


def result_metrics(records, overhead, resamples=2000, seed=0):
    """EXP-002 metrics per language, gaps with 95% bootstrap intervals resampling problems."""
    by = {}
    for r in records:
        by.setdefault(r["name"], {}).setdefault(r["lang"], []).append(r)
    names = sorted(n for n, v in by.items() if "aleph" in v and "python" in v)

    def stats(sample):
        acc = {lang: {"n": 0, "first": 0, "final": 0, "rounds": 0, "tokens": 0} for lang in ("aleph", "python")}
        for n in sample:
            for lang, recs in by[n].items():
                a = acc[lang]
                for r in recs:
                    a["n"] += 1
                    a["first"] += r["first"] == "pass"
                    a["final"] += r["final"] == "pass"
                    a["rounds"] += len(r["rounds"])
                    a["tokens"] += sum(session_tokens(r, overhead))
        out = {}
        for lang, a in acc.items():
            out[lang] = {"pass@1": a["first"] / a["n"], "pass_final": a["final"] / a["n"],
                         "rounds": a["rounds"] / a["n"], "tokens_per_solved": a["tokens"] / max(a["final"], 1)}
        return out

    point = stats(names)
    rng = random.Random(seed)
    boots = [stats([rng.choice(names) for _ in names]) for _ in range(resamples)]

    def ci(f):
        vals = sorted(f(b) for b in boots)
        return [round(vals[int(0.025 * len(vals))], 4), round(vals[int(0.975 * len(vals)) - 1], 4)]

    metrics = []
    for key, unit in (("pass@1", "share"), ("pass_final", "share"), ("rounds", "rounds"),
                      ("tokens_per_solved", "tokens")):
        for lang in ("python", "aleph"):
            metrics.append({"name": key, "value": round(point[lang][key], 4), "unit": unit, "condition": lang})
        if unit == "tokens":
            metrics.append({"name": key + " ratio aleph/python",
                            "value": round(point["aleph"][key] / point["python"][key], 3), "unit": "ratio",
                            "ci95": ci(lambda b: b["aleph"][key] / b["python"][key])})
        else:
            metrics.append({"name": key + " gap aleph-python", "value": round(point["aleph"][key] - point["python"][key], 4),
                            "unit": unit, "ci95": ci(lambda b: b["aleph"][key] - b["python"][key])})
    for lang in ("python", "aleph"):
        rows = [session_tokens(r, overhead) for n in names for r in by[n][lang]]
        for i, part in enumerate(("net input", "output", "reasoning")):
            metrics.append({"name": f"median {part} tokens per session", "value": statistics.median(t[i] for t in rows),
                            "unit": "tokens", "condition": lang})
        walls = [sum(rd["wall_s"] for rd in r["rounds"]) for n in names for r in by[n][lang]]
        metrics.append({"name": "median wall-clock per session", "value": statistics.median(walls), "unit": "s",
                        "condition": lang})
    return metrics


def main(argv=None):
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("--model", required=True)
    r.add_argument("--lang", required=True, choices=["aleph", "python"])
    r.add_argument("--out", required=True)
    r.add_argument("--rep", type=int, default=1)
    r.add_argument("--rounds", type=int, default=5)
    r.add_argument("--only")
    r.add_argument("--limit", type=int)
    b = sub.add_parser("build-python")
    b.add_argument("mple_dir")
    c = sub.add_parser("calibrate")
    c.add_argument("--model", required=True)
    s = sub.add_parser("summary")
    s.add_argument("log")
    s.add_argument("--overhead", type=int, default=0)
    m = sub.add_parser("metrics")
    m.add_argument("log")
    m.add_argument("--overhead", type=int, default=0)
    a = p.parse_args(argv)
    if a.cmd == "run":
        cmd_run(a)
    elif a.cmd == "build-python":
        build_python(a.mple_dir)
    elif a.cmd == "calibrate":
        cmd_calibrate(a)
    elif a.cmd == "metrics":
        print(json.dumps(result_metrics(list(map(json.loads, open(a.log))), a.overhead), indent=1))
    else:
        print("\n".join(summarize(map(json.loads, open(a.log)), a.overhead)))


if __name__ == "__main__":
    main()
