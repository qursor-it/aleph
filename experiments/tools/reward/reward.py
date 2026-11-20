#!/usr/bin/env python3
"""Reward function for EXP-004 RL training (T-042).

Usage:
  reward.py TASK_DIR PROGRAM_FILE [--lang aleph|python] [--json] [--time-limit S]

Python 3 standard library only, offline, never calls a model. See README.md.
"""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

SUITE_DIR = pathlib.Path(__file__).resolve().parent.parent / "suite"
REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
CHECK_TIMEOUT_S = 60
if str(SUITE_DIR) not in sys.path:
    sys.path.insert(0, str(SUITE_DIR))
import suite  # noqa: E402  (path set above)


def _aleph_binary(aleph):
    return aleph or os.environ.get("ALEPH") or str(REPO_ROOT / "target" / "debug" / "aleph")


def _result(**fields):
    base = {"reward": 0.0, "tests_passed": 0, "tests_total": 0, "check_ok": None,
            "capability_violation": False, "timed_out": False, "detail": ""}
    base.update(fields)
    return base


def _aleph_check(program, aleph):
    command = [aleph, "check", "--json", str(program)]
    try:
        proc = subprocess.run(command, capture_output=True, timeout=CHECK_TIMEOUT_S,
                              stdin=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        return False, True, "aleph check timed out"
    except OSError as error:
        return False, False, f"cannot run aleph: {error}"
    out = (proc.stdout + proc.stderr).decode("utf-8", errors="replace").strip()
    return proc.returncode == 0, False, out[:2000]


def _capability_trap(task_dir, program, aleph, tests, limit):
    """Re-run the failed tests and look for the R003 trap on stderr.

    suite.run_tests does not keep stderr when stdout already differs.
    """
    for test in tests:
        with tempfile.TemporaryDirectory() as directory:
            work = pathlib.Path(directory)
            if (task_dir / "inputs").is_dir():
                shutil.copytree(task_dir / "inputs", work, dirs_exist_ok=True)
            for rel, content in test.get("files", {}).items():
                suite._write_file(work, rel, content)
            try:
                proc = subprocess.run(suite._command("aleph", program, test.get("args", []), aleph),
                                      cwd=work, capture_output=True, timeout=limit,
                                      input=test.get("stdin", "").encode("utf-8"))
            except (subprocess.TimeoutExpired, OSError):
                continue
            if proc.returncode == 70 and b'"R003"' in proc.stderr:
                return True
    return False


def reward(program_text, task_dir, lang="aleph", aleph=None, time_limit=None):
    """Score one program against one task folder; see README.md for the definition."""
    task_dir = pathlib.Path(task_dir).resolve()
    if lang not in ("aleph", "python"):
        raise ValueError(f"unknown language {lang!r}; expected aleph or python")
    aleph = _aleph_binary(aleph)
    with tempfile.TemporaryDirectory() as directory:
        work = pathlib.Path(directory)
        program = work / ("program.aph" if lang == "aleph" else "program.py")
        program.write_text(program_text, encoding="utf-8")
        # A copy of the task, so the time limit can be overridden without touching the original.
        task_copy = work / "task"
        task_copy.mkdir()
        for name in ("task.json", "tests.json", "inputs"):
            src = task_dir / name
            if src.is_dir():
                shutil.copytree(src, task_copy / name)
            elif src.is_file():
                shutil.copy(src, task_copy / name)
        task = suite.load_json(task_copy / "task.json")
        limit = time_limit if time_limit is not None else task.get("time_limit_s", suite.DEFAULT_TIME_LIMIT)
        task["time_limit_s"] = limit
        (task_copy / "task.json").write_text(json.dumps(task), encoding="utf-8")
        tests = suite.load_json(task_copy / "tests.json")

        check_ok = None
        if lang == "aleph":
            check_ok, check_timeout, output = _aleph_check(program, aleph)
            if not check_ok:
                return _result(tests_total=len(tests), check_ok=False, timed_out=check_timeout,
                               detail=f"aleph check failed: {output}")

        results = suite.run_tests(task_copy, lang, program, aleph=aleph, log_access=(lang == "python"))
        passed = sum(1 for r in results if r["passed"])
        timed_out = any(r["detail"].startswith("timeout") for r in results)
        violation = False
        if lang == "python":
            violation = any(r.get("outside") for r in results)
        elif passed < len(results):
            failed = [t for t, r in zip(tests, results) if not r["passed"]]
            violation = _capability_trap(task_copy, program, aleph, failed, limit)
        value = 0.0 if violation else passed / len(results)
        if violation:
            detail = "capability violation" if lang == "aleph" else "file access outside declared files"
        else:
            detail = "; ".join(f"{r['name']}: {r['detail']}" for r in results if not r["passed"]) or "ok"
        return _result(reward=value, tests_passed=passed, tests_total=len(results), check_ok=check_ok,
                       capability_violation=violation, timed_out=timed_out, detail=detail)


def reward_many(items, workers=1):
    """Score a batch; items are dicts of reward() keyword arguments.

    Each item needs program_text and task_dir; lang, aleph and time_limit are
    optional. Returns one dict per item, in order. A failure inside one item
    gives reward 0 with the error in detail; it never raises. workers is
    accepted for trainer compatibility but evaluation is sequential. reward()
    keeps no shared state (private temp dirs), so callers may also run it from
    their own threads or processes.
    """
    out = []
    for item in items:
        try:
            out.append(reward(**item))
        except Exception as error:  # a bad program must never crash the trainer
            out.append(_result(detail=f"error: {error}"))
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("task_dir")
    parser.add_argument("program_file")
    parser.add_argument("--lang", choices=["aleph", "python"], default="aleph")
    parser.add_argument("--time-limit", type=float, default=None)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    text = pathlib.Path(args.program_file).read_text(encoding="utf-8")
    result = reward(text, args.task_dir, lang=args.lang, time_limit=args.time_limit)
    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print(" ".join(f"{k}={v}" for k, v in result.items()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
