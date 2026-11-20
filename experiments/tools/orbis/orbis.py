#!/usr/bin/env python3
"""Orbis generator for EXP-004 (T-041): task statements to verified programs.

Usage:
  orbis.py generate STATEMENTS.jsonl --model BACKEND:NAME --out-dir DIR
      [--attempts 3] [--cap-eur X --prices P.json]
      [--dedup-against PATH ...] [--threshold 0.5] [--llms PATH]
      [--max-tokens N] [--seed N]

INPUT is JSONL with one object per task statement:
  {"id", "statement", "category", "tests" (optional), "files" (optional)}

Outputs, under DIR:
  programs/<id>.aph   canonical, check-clean, tests-passing programs
  index.jsonl         one line per kept program
  repairs.jsonl       {id, broken, diagnostics, fixed} per repaired attempt
  summary.json        counts plus dropped statements and their nearest match

Python 3 standard library only, offline. The model comes from the EXP-002
harness backends; the tests use `fake:`. Never run a real backend in tests.
See README.md.
"""
import argparse
import contextlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]
HARNESS_DIR = HERE.parent / "harness"
SUITE_DIR = HERE.parent / "suite"
DEFAULT_ALEPH = REPO_ROOT / "target" / "debug" / "aleph"
DEFAULT_THRESHOLD = 0.5
NGRAM = 5
CHECK_TIMEOUT_S = 60

for _path in (HARNESS_DIR, SUITE_DIR):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))
import harness  # noqa: E402  (sibling tool, path set above)
import suite  # noqa: E402  (sibling tool, path set above)


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def load_statements(path):
    """Read the JSONL statements file; blank lines are ignored."""
    statements = []
    with open(path, encoding="utf-8") as handle:
        for number, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except ValueError as error:
                raise SystemExit(f"{path}:{number}: invalid JSON: {error}")
            if not isinstance(record, dict) or not record.get("id") or not record.get("statement"):
                raise SystemExit(f"{path}:{number}: needs a non-empty id and statement")
            statements.append(record)
    return statements


def normalize_words(text):
    """Words of a statement, lowercased, punctuation and layout removed."""
    return re.findall(r"[a-z0-9]+", (text or "").casefold())


def word_ngrams(text, n=NGRAM):
    """Set of normalised word n-grams; a shorter text is one whole n-gram."""
    words = normalize_words(text)
    if not words:
        return set()
    if len(words) < n:
        return {tuple(words)}
    return {tuple(words[index:index + n]) for index in range(len(words) - n + 1)}


def jaccard(left, right):
    if not left and not right:
        return 0.0
    union = left | right
    return len(left & right) / len(union) if union else 0.0


def _source_entries(path):
    """Statements in one dedup source; never reads tests or reference code."""
    path = pathlib.Path(path)
    entries = []
    if path.is_dir():
        for folder in sorted(item for item in path.iterdir() if item.is_dir()):
            task_path = folder / "task.json"
            if not task_path.is_file():
                continue
            try:
                task = load_json(task_path)
            except (OSError, ValueError):
                continue
            statement = task.get("statement") if isinstance(task, dict) else None
            if isinstance(statement, str) and statement.strip():
                entries.append({
                    "id": str(task.get("id") or folder.name),
                    "statement": statement,
                    "source": str(task_path),
                })
        return entries

    with open(path, encoding="utf-8") as handle:
        for number, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except ValueError:
                continue
            if not isinstance(record, dict):
                continue
            statement = record.get("statement") or record.get("prompt")
            if not isinstance(statement, str) or not statement.strip():
                continue
            name = record.get("id") or record.get("name") or number
            entries.append({
                "id": str(name),
                "statement": statement,
                "source": f"{path}:{number}",
            })
    return entries


def load_dedup_sources(paths):
    sources = []
    for path in paths or ():
        sources.extend(_source_entries(path))
    return sources


def deduplicate(statements, sources, threshold=DEFAULT_THRESHOLD):
    """Drop statements close to a source; return (kept, dropped)."""
    prepared = [(entry, word_ngrams(entry.get("statement", ""))) for entry in sources]
    kept, dropped = [], []
    for statement in statements:
        grams = word_ngrams(statement.get("statement", ""))
        nearest, best = None, 0.0
        for entry, source_grams in prepared:
            score = jaccard(grams, source_grams)
            if score > best:
                nearest, best = entry, score
        if nearest is not None and best >= threshold:
            dropped.append({
                "id": statement["id"],
                "nearest_id": nearest["id"],
                "nearest_source": nearest["source"],
                "score": round(best, 6),
            })
        else:
            kept.append(statement)
    return kept, dropped


def task_from_record(record):
    files = record.get("files")
    if not isinstance(files, dict):
        files = {}
    read = files.get("read") if isinstance(files.get("read"), list) else []
    write = files.get("write") if isinstance(files.get("write"), list) else []
    return {
        "id": str(record.get("id")),
        "category": record.get("category") or "algorithm",
        "statement": record.get("statement", ""),
        "bash_natural": False,
        "files": {"read": list(read), "write": list(write)},
        "time_limit_s": record.get("time_limit_s", suite.DEFAULT_TIME_LIMIT),
    }


@contextlib.contextmanager
def _task_dir(record):
    """A temporary EXP-002 task folder for prompts, checks and hidden tests."""
    with tempfile.TemporaryDirectory() as directory:
        task = pathlib.Path(directory)
        (task / "task.json").write_text(
            json.dumps(task_from_record(record)), encoding="utf-8")
        tests = record.get("tests")
        (task / "tests.json").write_text(
            json.dumps(tests if isinstance(tests, list) else []), encoding="utf-8")
        yield task


def _binary(aleph):
    """Resolve a binary given by path, so subprocess cwd changes are safe."""
    if os.sep in aleph or (os.altsep and os.altsep in aleph):
        return str(pathlib.Path(aleph).resolve())
    return aleph


def _parse_diagnostics(output):
    diagnostics = []
    for line in output.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            diagnostic = json.loads(line)
        except ValueError:
            continue
        if isinstance(diagnostic, dict) and "code" in diagnostic:
            diagnostics.append(diagnostic)
    return diagnostics


def _synthetic_error(message, program="program.aph"):
    return {"code": "CRASH", "severity": "error", "file": program,
            "line": 1, "col": 1, "msg": message}


def run_check(program_text, task, aleph):
    """Run `aleph check --json` on program.aph in task; parse its diagnostics."""
    program = task / "program.aph"
    text = program_text if program_text.endswith("\n") else program_text + "\n"
    program.write_text(text, encoding="utf-8")
    binary = _binary(aleph)
    try:
        proc = subprocess.run(
            [binary, "check", "--json", "program.aph"], cwd=task,
            capture_output=True, timeout=CHECK_TIMEOUT_S, stdin=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        return [_synthetic_error("aleph check timed out")], None
    except OSError as error:
        return [_synthetic_error(f"cannot run aleph: {error}")], None
    output = (proc.stdout + proc.stderr).decode("utf-8", errors="replace")
    diagnostics = _parse_diagnostics(proc.stdout.decode("utf-8", errors="replace"))
    if proc.returncode != 0 and not any(
            diagnostic.get("severity") == "error" for diagnostic in diagnostics):
        diagnostics.append(_synthetic_error("aleph check failed without diagnostics"))
    return diagnostics, text


def _fmt(program_text, task, aleph):
    program = task / "program.aph"
    program.write_text(
        program_text if program_text.endswith("\n") else program_text + "\n",
        encoding="utf-8")
    try:
        proc = subprocess.run(
            [_binary(aleph), "fmt", "program.aph"], cwd=task,
            capture_output=True, timeout=CHECK_TIMEOUT_S, stdin=subprocess.DEVNULL)
    except (subprocess.TimeoutExpired, OSError):
        return None
    if proc.returncode != 0:
        return None
    return proc.stdout.decode("utf-8", errors="replace")


def _final_newline(text):
    return text if text.endswith("\n") else text + "\n"


def evaluate(program_text, record, aleph):
    """Check one attempt and, when requested, run the statement's hidden tests."""
    with _task_dir(record) as task:
        diagnostics, text = run_check(program_text, task, aleph)
        errors = [item for item in diagnostics if item.get("severity") == "error"]
        canonical = None
        for diagnostic in diagnostics:
            if diagnostic.get("code") == "F001" and isinstance(
                    diagnostic.get("canonical"), str):
                canonical = diagnostic["canonical"]
                break
        results = None
        if not errors and isinstance(record.get("tests"), list) and record["tests"]:
            try:
                results = suite.run_tests(task, "aleph", task / "program.aph", aleph=aleph)
            except (OSError, ValueError, KeyError) as error:
                results = [{"name": "tests", "passed": False,
                            "detail": f"cannot run: {error}"}]
        if not errors:
            if canonical is None:
                formatted = _fmt(text, task, aleph)
                canonical = formatted if formatted is not None else text
            canonical = _final_newline(canonical)
        passed = not errors and (results is None or all(
            result.get("passed") for result in results))
        return {
            "passed": passed,
            "diagnostics": diagnostics,
            "errors": errors,
            "canonical": canonical,
            "results": results,
        }


def build_messages(record, llms_path):
    with _task_dir(record) as task:
        system, user = harness.build_prompt("aleph-llms", task, llms_path=llms_path)
    return [{"role": "system", "content": system},
            {"role": "user", "content": user}]


def _truncate(text, limit=500):
    text = text or ""
    return text if len(text) <= limit else text[:limit]


def format_feedback(diagnostics, results, tests):
    """JSON diagnostics plus failing tests, in the harness feedback shape."""
    lines = ["Static check: failed" if any(
        item.get("severity") == "error" for item in diagnostics) else "Static check: ok"]
    if diagnostics:
        lines.append("Diagnostics (JSON, one per line):")
        lines.extend(json.dumps(item, ensure_ascii=False) for item in diagnostics)
    if results is not None:
        expected = {test.get("name", "?"): test.get("stdout", "") for test in tests}
        failed = [result for result in results if not result.get("passed")]
        if failed:
            lines.append(f"Failing tests ({len(failed)} of {len(results)}):")
            for result in failed:
                name = result.get("name", "?")
                lines.append(f"- {name}")
                lines.append("  expected stdout: " + _truncate(repr(expected.get(name, ""))))
                lines.append("  actual: " + _truncate(result.get("detail", "")))
        else:
            lines.append("All hidden tests passed.")
    lines.append("Answer with one fenced code block containing the complete program.")
    return "\n".join(lines)


def _fail(message):
    print(message, file=sys.stderr)
    raise SystemExit(2)


def _backend_name(model):
    if ":" not in model:
        _fail("--model must be BACKEND:NAME, e.g. fake:answers.json")
    name = model.split(":", 1)[0]
    if name not in ("fake", "openai"):
        _fail(f"unknown backend {name!r}; expected fake or openai")
    return name


def validate_budget(model, cap_eur, prices_path):
    """Fake backends may run without a cap; every other backend may not."""
    backend = _backend_name(model)
    if backend != "fake" and cap_eur is None:
        _fail("--cap-eur is required for non-fake backends")
    if backend != "fake" and prices_path is None:
        _fail("--prices is required for non-fake backends")
    if cap_eur is not None and prices_path is None:
        _fail("--prices is required with --cap-eur")


def generate(statements_path, model, out_dir, attempts=3, cap_eur=None,
             prices_path=None, dedup_against=(), threshold=DEFAULT_THRESHOLD,
             aleph=None, llms_path=None, max_tokens=harness.DEFAULT_MAX_TOKENS,
             seed=None):
    """Generate verified programs and repair triples; write DIR and return summary."""
    validate_budget(model, cap_eur, prices_path)
    if attempts < 1:
        _fail("--attempts must be at least 1")

    statements = load_statements(statements_path)
    sources = load_dedup_sources(dedup_against)
    kept_statements, dropped = deduplicate(statements, sources, threshold=threshold)
    for item in dropped:
        print(f"dropped {item['id']} ~ {item['nearest_id']} {item['score']:.3f}",
              file=sys.stderr)

    prices = load_json(prices_path) if prices_path else {}
    price = harness.price_for(prices, model) if prices else None
    backend = harness.make_backend(model, max_tokens=max_tokens)
    aleph = aleph or harness.resolve_aleph()
    llms_path = llms_path or harness.DEFAULT_LLMS

    out_dir = pathlib.Path(out_dir)
    programs_dir = out_dir / "programs"
    programs_dir.mkdir(parents=True, exist_ok=True)

    index, repairs, failed = [], [], []
    spend = 0.0
    stopped = False

    for record in kept_statements:
        statement_id = str(record["id"])
        messages = build_messages(record, llms_path)
        broken_attempts = []
        kept_result = None

        for number in range(1, attempts + 1):
            prompt_chars = sum(len(message["content"]) for message in messages)
            worst = (harness.worst_case_cost(prompt_chars, max_tokens, price)
                     if price is not None else 0.0)
            if cap_eur is not None and spend + worst > cap_eur:
                stopped = True
                break

            answer = backend.complete(messages, seed=seed)
            raw = answer.get("text") or ""
            usage = harness.normalize_usage(answer.get("usage"))
            if price is not None:
                spend += harness.round_cost(usage, price)
            program_text = harness.extract_code_block(raw)
            outcome = evaluate(program_text, record, aleph)

            if outcome["passed"]:
                kept_result = outcome
                break

            broken_attempts.append({
                "broken": program_text,
                "diagnostics": outcome["diagnostics"],
            })
            if number < attempts:
                results = outcome["results"]
                tests = record.get("tests") if isinstance(record.get("tests"), list) else []
                messages.append({"role": "assistant", "content": raw})
                messages.append({
                    "role": "user",
                    "content": format_feedback(outcome["diagnostics"], results, tests),
                })

        if kept_result is None:
            if stopped:
                break
            failed.append({
                "id": statement_id,
                "attempts": len(broken_attempts),
                "diagnostics": broken_attempts[-1]["diagnostics"] if broken_attempts else [],
            })
            continue

        canonical = kept_result["canonical"]
        relative_path = f"programs/{statement_id}.aph"
        (out_dir / relative_path).write_text(canonical, encoding="utf-8")
        for broken in broken_attempts:
            repairs.append({
                "id": statement_id,
                "broken": broken["broken"],
                "diagnostics": broken["diagnostics"],
                "fixed": canonical,
            })
        index.append({
            "id": statement_id,
            "statement": record.get("statement", ""),
            "category": record.get("category") or "algorithm",
            "path": relative_path,
        })

    with open(out_dir / "index.jsonl", "w", encoding="utf-8") as handle:
        for item in index:
            handle.write(json.dumps(item, ensure_ascii=False) + "\n")
    with open(out_dir / "repairs.jsonl", "w", encoding="utf-8") as handle:
        for item in repairs:
            handle.write(json.dumps(item, ensure_ascii=False) + "\n")

    cost = round(spend, 9)
    summary = {
        "model": model,
        "statements": len(statements),
        "kept": len(index),
        "dropped": len(dropped),
        "failed": len(failed),
        "repairs": len(repairs),
        "cost": cost,
        "cost_eur": cost,
        "stopped": stopped,
        "kept_ids": [item["id"] for item in index],
        "dropped_details": dropped,
        "failed_details": failed,
    }
    (out_dir / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8")
    return summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    run = sub.add_parser("generate", help="generate programs from statements")
    run.add_argument("statements", help="statements JSONL")
    run.add_argument("--model", required=True, help="fake:ANSWERS.json or openai:MODEL")
    run.add_argument("--out-dir", required=True)
    run.add_argument("--attempts", type=int, default=3)
    run.add_argument("--cap-eur", type=float, default=None,
                     help="spending cap in euro; required for non-fake backends")
    run.add_argument("--prices", default=None)
    run.add_argument("--dedup-against", nargs="+", action="extend", default=[],
                     help="EXP-002 suite dirs and/or JSONL files with prompt/statement")
    run.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD,
                     help="drop at this word n-gram Jaccard similarity or above")
    run.add_argument("--llms", default=str(harness.DEFAULT_LLMS))
    run.add_argument("--max-tokens", type=int, default=harness.DEFAULT_MAX_TOKENS)
    run.add_argument("--seed", type=int, default=None)
    run.set_defaults(func=_cmd_generate)

    args = parser.parse_args(argv)
    return args.func(args)


def _cmd_generate(args):
    summary = generate(
        args.statements, args.model, args.out_dir,
        attempts=args.attempts, cap_eur=args.cap_eur, prices_path=args.prices,
        dedup_against=args.dedup_against, threshold=args.threshold,
        llms_path=args.llms, max_tokens=args.max_tokens, seed=args.seed,
    )
    print(json.dumps(summary, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
