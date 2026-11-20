#!/usr/bin/env python3
"""EXP-002 A/B harness: run one condition on one task.

Usage:
  harness.py run --task TASK_DIR --condition CONDITION --model BACKEND:NAME
      --cap-eur X --prices PRICES.json --out LOG.jsonl [--rounds 5]
      [--examples DIR] [--seed N] [--llms PATH] [--max-tokens N]
  harness.py summary LOG.jsonl

CONDITION is one of python, bash, aleph-llms, aleph-llms-examples.
BACKEND is fake:<answers.json> (offline tests) or openai:<model> (never run in
tests). Python 3 standard library only. See README.md.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time
import urllib.request
import uuid

HERE = pathlib.Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]
PROMPTS_DIR = HERE / "prompts"
SUITE_DIR = HERE.parent / "suite"
DEFAULT_LLMS = REPO_ROOT / "llms.txt"
DEFAULT_ALEPH = REPO_ROOT / "target" / "debug" / "aleph"
DEFAULT_MAX_TOKENS = 4096

sys.path.insert(0, str(SUITE_DIR))
import suite  # noqa: E402  (sibling tool, imported after the path is set)

CONDITIONS = {
    "python": {"lang": "python", "ext": ".py", "examples": False},
    "bash": {"lang": "bash", "ext": ".sh", "examples": False},
    "aleph-llms": {"lang": "aleph", "ext": ".aph", "examples": False},
    "aleph-llms-examples": {"lang": "aleph", "ext": ".aph", "examples": True},
}


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def read_records(path):
    path = pathlib.Path(path)
    if not path.exists():
        return []
    records = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                try:
                    records.append(json.loads(line))
                except ValueError:
                    continue
    return records


def append_records(path, records):
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")
        handle.flush()


def previous_spend(path):
    """Sum the costs of every earlier round in the log (the cap is shared)."""
    return sum(float(record.get("cost_eur") or 0) for record in read_records(path))


def truncate(text, limit):
    text = text or ""
    return text if len(text) <= limit else text[:limit]


def extract_code_block(text):
    """The last fenced block in the answer, or the whole answer if none."""
    blocks = re.findall(r"```[^\n]*\n(.*?)```", text or "", re.S)
    return blocks[-1].strip("\n") if blocks else (text or "").strip()


def resolve_aleph():
    return os.environ.get("ALEPH") or str(DEFAULT_ALEPH)


def load_examples(examples_dir, exclude_task_dir, limit=5):
    """Up to `limit` solved examples, never the task currently being run."""
    if not examples_dir:
        return []
    examples_dir = pathlib.Path(examples_dir)
    excluded = pathlib.Path(exclude_task_dir).resolve()
    try:
        excluded_id = load_json(excluded / "task.json").get("id") or excluded.name
    except (OSError, ValueError):
        excluded_id = excluded.name
    examples = []
    for folder in sorted(p for p in examples_dir.iterdir() if p.is_dir()):
        if folder.resolve() == excluded:
            continue
        task_path = folder / "task.json"
        solution_path = folder / "ref" / "solution.aph"
        try:
            task = load_json(task_path)
            solution = solution_path.read_text(encoding="utf-8")
        except (OSError, ValueError):
            continue
        task_id = task.get("id") or folder.name
        if task_id == excluded_id:
            continue
        examples.append({
            "id": task_id,
            "statement": task.get("statement", ""),
            "solution": solution.rstrip("\n"),
        })
        if len(examples) == limit:
            break
    return examples


def format_examples(examples):
    if not examples:
        return "(no examples)"
    parts = []
    for example in examples:
        parts.append(
            f"### Example {example['id']}\n{example['statement'].strip()}\n\n"
            f"```aleph\n{example['solution']}\n```"
        )
    return "\n\n".join(parts)


def _render(text, values):
    for key, value in values.items():
        text = text.replace("{" + key + "}", value)
    return text


def build_prompt(condition, task_dir, llms_path=None, examples_dir=None):
    """Return (system, user) from the published prompts/<condition>.* files."""
    task_dir = pathlib.Path(task_dir).resolve()
    task = load_json(task_dir / "task.json")
    read_files = task.get("files", {}).get("read", [])
    write_files = task.get("files", {}).get("write", [])
    values = {
        "statement": task.get("statement", "").strip(),
        "read_files": ", ".join(read_files) if read_files else "(none)",
        "write_files": ", ".join(write_files) if write_files else "(none)",
    }
    if condition.startswith("aleph"):
        llms_path = pathlib.Path(llms_path or DEFAULT_LLMS)
        values["llms"] = llms_path.read_text(encoding="utf-8").rstrip("\n")
    if CONDITIONS[condition]["examples"]:
        examples = load_examples(examples_dir, task_dir) if examples_dir else []
        values["examples"] = format_examples(examples)
    system = _render((PROMPTS_DIR / f"{condition}.system.txt").read_text(encoding="utf-8"), values)
    user = _render((PROMPTS_DIR / f"{condition}.user.txt").read_text(encoding="utf-8"), values)
    return system, user


def static_check(lang, program_path, aleph):
    if lang == "aleph":
        command = [aleph, "check", "--json", str(program_path)]
    elif lang == "python":
        command = ["python3", "-m", "py_compile", str(program_path)]
    elif lang == "bash":
        command = ["bash", "-n", str(program_path)]
    else:  # pragma: no cover - guarded by CONDITION choices
        raise ValueError(f"unknown language {lang!r}")
    try:
        proc = subprocess.run(command, capture_output=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as error:
        return False, f"cannot run checker: {error}"
    output = (proc.stdout + proc.stderr).decode("utf-8", errors="replace").strip()
    return proc.returncode == 0, output


def _expected_stdout(tests):
    return {test.get("name", "?"): test.get("stdout", "") for test in tests}


def format_feedback(check_ok, check_output, results, tests):
    """The same feedback shape for every language (EXP-002 Setup)."""
    lines = [f"Static check: {'ok' if check_ok else 'failed'}"]
    if check_output:
        lines.append("Checker output:")
        lines.append(truncate(check_output, 2000))
    if results is not None:
        expected = _expected_stdout(tests)
        failed = [result for result in results if not result.get("passed")]
        if failed:
            lines.append(f"Failing tests ({len(failed)} of {len(results)}):")
            for result in failed:
                lines.append(f"- {result.get('name', '?')}")
                lines.append(
                    "  expected stdout: " + truncate(repr(expected.get(result.get("name"), "")), 500))
                lines.append("  actual: " + truncate(result.get("detail", ""), 500))
        else:
            lines.append("All hidden tests passed.")
    lines.append("Answer with one fenced code block containing the complete program.")
    return "\n".join(lines)


def normalize_usage(usage):
    usage = usage or {}
    cached = int(usage.get("cached_input") or 0)
    total_input = int(usage.get("input") or 0)
    return {
        "new_input": max(total_input - cached, 0),
        "cached_input": cached,
        "output": int(usage.get("output") or 0),
        "reasoning": int(usage.get("reasoning") or 0),
    }


def price_for(prices, model):
    """Prices by full --model, by NAME after the backend, or by 'default'."""
    if model in prices:
        return prices[model]
    name = model.split(":", 1)[1] if ":" in model else model
    if name in prices:
        return prices[name]
    if "default" in prices:
        return prices["default"]
    raise SystemExit(f"no price for model {model!r} in the prices file")


def round_cost(usage, price):
    cached_price = price.get("cached_input")
    if cached_price is None:
        cached_price = price.get("input", 0)
    return (
        usage["new_input"] * price.get("input", 0)
        + usage["cached_input"] * cached_price
        + (usage["output"] + usage["reasoning"]) * price.get("output", 0)
    ) / 1_000_000


def worst_case_cost(prompt_chars, max_tokens, price):
    prompt_tokens = prompt_chars / 4
    return (prompt_tokens * price.get("input", 0) + max_tokens * price.get("output", 0)) / 1_000_000


class FakeBackend:
    """Replay a JSON list of answers, offline. Used only by the tests."""

    def __init__(self, path):
        data = load_json(path)
        if isinstance(data, dict):
            data = data.get("answers", [])
        if not isinstance(data, list):
            raise ValueError("fake answers file must be a JSON list")
        self.answers = data
        self.index = 0
        self.calls = []

    def complete(self, messages, seed=None):
        self.calls.append({"messages": [dict(message) for message in messages], "seed": seed})
        if self.index >= len(self.answers):
            raise RuntimeError("fake backend ran out of scripted answers")
        entry = self.answers[self.index]
        self.index += 1
        if isinstance(entry, str):
            text, usage = entry, {}
        else:
            text = entry.get("text", "")
            usage = entry.get("usage", {})
        return {"text": text, "usage": dict(usage)}


class OpenAIBackend:
    """OpenAI-compatible chat completions. Implemented but never run in tests."""

    def __init__(self, model, max_tokens=DEFAULT_MAX_TOKENS, timeout=600):
        self.model = model
        self.max_tokens = max_tokens
        self.timeout = timeout

    def complete(self, messages, seed=None):
        base_url = os.environ.get("OPENAI_BASE_URL")
        api_key = os.environ.get("OPENAI_API_KEY")
        if not base_url:
            raise RuntimeError("set OPENAI_BASE_URL")
        if not api_key:
            raise RuntimeError("set OPENAI_API_KEY")
        body = {"model": self.model, "messages": messages, "max_tokens": self.max_tokens}
        if seed is not None:
            body["seed"] = seed
        request = urllib.request.Request(
            base_url.rstrip("/") + "/chat/completions",
            data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json",
                     "Authorization": f"Bearer {api_key}"},
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=self.timeout) as response:
            data = json.loads(response.read().decode("utf-8"))
        message = data["choices"][0]["message"]
        usage = data.get("usage") or {}
        prompt_details = usage.get("prompt_tokens_details") or {}
        completion_details = usage.get("completion_tokens_details") or {}
        return {
            "text": message.get("content") or "",
            "usage": {
                "input": usage.get("prompt_tokens") or 0,
                "cached_input": prompt_details.get("cached_tokens")
                or usage.get("prompt_cache_hit_tokens") or 0,
                "output": usage.get("completion_tokens") or 0,
                "reasoning": completion_details.get("reasoning_tokens") or 0,
            },
        }


def make_backend(model, max_tokens=DEFAULT_MAX_TOKENS):
    if ":" not in model:
        raise SystemExit("--model must be BACKEND:NAME, e.g. fake:answers.json")
    backend, name = model.split(":", 1)
    if backend == "fake":
        return FakeBackend(name)
    if backend == "openai":
        return OpenAIBackend(name, max_tokens=max_tokens)
    raise SystemExit(f"unknown backend {backend!r}; expected fake or openai")


def run_task(task_dir, condition, backend, model, cap_eur, prices, out_path,
             rounds=5, examples_dir=None, seed=None, llms_path=None,
             max_tokens=DEFAULT_MAX_TOKENS, aleph=None):
    """Run the round loop for one task and condition; write the log records."""
    task_dir = pathlib.Path(task_dir).resolve()
    task = load_json(task_dir / "task.json")
    tests = load_json(task_dir / "tests.json")
    task_id = task.get("id") or task_dir.name
    lang = CONDITIONS[condition]["lang"]
    ext = CONDITIONS[condition]["ext"]
    aleph = aleph or resolve_aleph()
    price = price_for(prices, model)
    spend = previous_spend(out_path)
    run_id = uuid.uuid4().hex[:12]
    system, user = build_prompt(condition, task_dir, llms_path=llms_path,
                                examples_dir=examples_dir)
    messages = [{"role": "system", "content": system},
                {"role": "user", "content": user}]
    records = []
    passed = False
    rounds_to_pass = None
    total_tokens = {"new_input": 0, "cached_input": 0, "output": 0, "reasoning": 0}
    total_cost = 0.0

    for number in range(1, rounds + 1):
        prompt_chars = sum(len(message["content"]) for message in messages)
        worst = worst_case_cost(prompt_chars, max_tokens, price)
        if spend + worst > cap_eur:
            records.append({
                "event": "stopped",
                "reason": "cap",
                "message": "stopped: cap",
                "run_id": run_id,
                "task": task_id,
                "condition": condition,
                "model": model,
                "seed": seed,
                "round": number,
                "spend_eur": round(spend, 9),
                "worst_case_eur": round(worst, 9),
                "cap_eur": cap_eur,
                "cost_eur": 0.0,
            })
            break

        started = time.monotonic()
        answer = backend.complete(messages, seed=seed)
        wall_s = time.monotonic() - started
        text = answer.get("text") or ""
        usage = normalize_usage(answer.get("usage"))
        cost = round_cost(usage, price)
        spend += cost
        total_cost += cost
        for key in total_tokens:
            total_tokens[key] += usage[key]
        program_text = extract_code_block(text)

        with tempfile.TemporaryDirectory() as directory:
            program_path = pathlib.Path(directory) / ("program" + ext)
            program_path.write_text(
                program_text if program_text.endswith("\n") else program_text + "\n",
                encoding="utf-8")
            check_ok, check_output = static_check(lang, program_path, aleph)
            try:
                results = suite.run_tests(task_dir, lang, program_path, aleph=aleph)
            except (OSError, ValueError, KeyError) as error:
                results = [{"name": "tests", "passed": False, "detail": f"cannot run: {error}"}]

        tests_total = len(results)
        tests_passed = sum(1 for result in results if result.get("passed"))
        passed = tests_total > 0 and tests_passed == tests_total
        records.append({
            "event": "round",
            "run_id": run_id,
            "task": task_id,
            "condition": condition,
            "model": model,
            "seed": seed,
            "round": number,
            "prompt_chars": prompt_chars,
            "usage": usage,
            "cost_eur": cost,
            "wall_s": round(wall_s, 3),
            "check_ok": check_ok,
            "tests_passed": tests_passed,
            "tests_total": tests_total,
            "passed": passed,
            "program": program_text,
        })
        messages.append({"role": "assistant", "content": text})
        if passed:
            rounds_to_pass = number
            break
        messages.append({
            "role": "user",
            "content": format_feedback(check_ok, check_output, results, tests),
        })

    records.append({
        "event": "summary",
        "run_id": run_id,
        "task": task_id,
        "condition": condition,
        "model": model,
        "seed": seed,
        "passed": passed,
        "rounds_to_pass": rounds_to_pass,
        "rounds": sum(1 for record in records if record.get("event") == "round"),
        "max_rounds": rounds,
        "total_tokens": total_tokens,
        "total_cost_eur": total_cost,
    })
    append_records(out_path, records)
    return records


def summarize(log_path):
    """Aggregate a log file: pass@1, pass@k, rounds, tokens and cost by condition."""
    runs = {}
    for record in read_records(log_path):
        run_id = record.get("run_id")
        if not run_id:
            continue
        run = runs.setdefault(run_id, {"rounds": [], "summary": None})
        if record.get("event") == "round":
            run["rounds"].append(record)
        elif record.get("event") == "summary":
            run["summary"] = record

    by_condition = {}
    for run in runs.values():
        rounds = sorted(run["rounds"], key=lambda record: record.get("round", 0))
        summary = run["summary"] or {}
        condition = summary.get("condition") or (rounds[0]["condition"] if rounds else "?")
        passed = summary.get("passed")
        if passed is None:
            passed = any(record.get("passed") for record in rounds)
        first_passed = any(record.get("round") == 1 and record.get("passed") for record in rounds)
        rounds_to_pass = summary.get("rounds_to_pass")
        if rounds_to_pass is None:
            passed_rounds = [record["round"] for record in rounds if record.get("passed")]
            rounds_to_pass = min(passed_rounds) if passed_rounds else None
        tokens = {"new_input": 0, "cached_input": 0, "output": 0, "reasoning": 0}
        cost = 0.0
        for record in rounds:
            usage = record.get("usage") or {}
            for key in tokens:
                tokens[key] += int(usage.get(key) or 0)
            cost += float(record.get("cost_eur") or 0)
        stats = by_condition.setdefault(condition, {
            "runs": 0,
            "solved": 0,
            "pass_at_1": 0.0,
            "pass_at_k": 0.0,
            "k": 0,
            "mean_rounds_to_pass": None,
            "tokens_per_solved": None,
            "cost_eur_per_solved": None,
            "total_tokens": {key: 0 for key in tokens},
            "total_cost_eur": 0.0,
            "_pass1": 0,
            "_rounds_to_pass": [],
            "_solved_tokens": {key: 0 for key in tokens},
            "_solved_cost": 0.0,
        })
        stats["runs"] += 1
        stats["k"] = max(stats["k"], int(summary.get("max_rounds") or len(rounds)))
        if first_passed:
            stats["_pass1"] += 1
        if passed:
            stats["solved"] += 1
            if rounds_to_pass is not None:
                stats["_rounds_to_pass"].append(rounds_to_pass)
            for key in tokens:
                stats["_solved_tokens"][key] += tokens[key]
            stats["_solved_cost"] += cost
        for key in tokens:
            stats["total_tokens"][key] += tokens[key]
        stats["total_cost_eur"] += cost

    for stats in by_condition.values():
        runs_count = stats["runs"]
        solved = stats["solved"]
        stats["pass_at_1"] = stats["_pass1"] / runs_count if runs_count else 0.0
        stats["pass_at_k"] = solved / runs_count if runs_count else 0.0
        if stats["_rounds_to_pass"]:
            stats["mean_rounds_to_pass"] = sum(stats["_rounds_to_pass"]) / len(stats["_rounds_to_pass"])
        if solved:
            stats["tokens_per_solved"] = {
                key: value / solved for key, value in stats["_solved_tokens"].items()}
            stats["cost_eur_per_solved"] = stats["_solved_cost"] / solved
        del stats["_pass1"], stats["_rounds_to_pass"]
        del stats["_solved_tokens"], stats["_solved_cost"]
    return {"runs": len(runs), "by_condition": by_condition}


def _cmd_run(args):
    if args.condition == "aleph-llms-examples" and not args.examples:
        raise SystemExit("--examples DIR is required for aleph-llms-examples")
    if args.rounds < 1:
        raise SystemExit("--rounds must be at least 1")
    backend = make_backend(args.model, max_tokens=args.max_tokens)
    prices = load_json(args.prices)
    run_task(
        args.task, args.condition, backend, args.model, args.cap_eur, prices,
        args.out, rounds=args.rounds, examples_dir=args.examples, seed=args.seed,
        llms_path=args.llms, max_tokens=args.max_tokens,
    )
    return 0


def _cmd_summary(args):
    print(json.dumps(summarize(args.log), indent=2))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description="EXP-002 A/B harness")
    sub = parser.add_subparsers(dest="command", required=True)

    run = sub.add_parser("run", help="run one condition on one task")
    run.add_argument("--task", required=True)
    run.add_argument("--condition", required=True, choices=sorted(CONDITIONS))
    run.add_argument("--model", required=True, help="fake:ANSWERS.json or openai:MODEL")
    run.add_argument("--cap-eur", type=float, required=True,
                     help="spending cap in euro; the run refuses to start without it")
    run.add_argument("--prices", required=True)
    run.add_argument("--out", required=True)
    run.add_argument("--rounds", type=int, default=5)
    run.add_argument("--examples")
    run.add_argument("--seed", type=int)
    run.add_argument("--llms", default=str(DEFAULT_LLMS))
    run.add_argument("--max-tokens", type=int, default=DEFAULT_MAX_TOKENS)
    run.set_defaults(func=_cmd_run)

    summary = sub.add_parser("summary", help="aggregate a log file")
    summary.add_argument("log")
    summary.set_defaults(func=_cmd_summary)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
