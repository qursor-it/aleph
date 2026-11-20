#!/usr/bin/env python3
"""Task-suite format for EXP-002: validate tasks and run their hidden tests.

Usage:
  suite.py validate SUITE_DIR [--refs]
  suite.py run TASK_DIR {python|bash|aleph} PROGRAM_FILE [--json] [--log-access]

Python 3 standard library only, offline. See README.md for the task format.
"""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

CATEGORIES = {"text", "files", "data", "json", "algorithm"}
LANGS = {"python": "python3", "bash": "bash", "aleph": None}
REF_LANG = {".py": "python", ".sh": "bash", ".aph": "aleph"}
DEFAULT_TIME_LIMIT = 10
SANDBOX_DIR = pathlib.Path(__file__).resolve().parent.parent / "sandbox"


def _sandbox_module():
    """Import the sibling sandbox tool lazily, only when access logging is on."""
    if str(SANDBOX_DIR) not in sys.path:
        sys.path.insert(0, str(SANDBOX_DIR))
    import sandbox  # noqa: E402  (path set above)
    return sandbox


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def default_aleph():
    return os.environ.get("ALEPH", "aleph")


def _safe_rel(path):
    """A relative path that cannot escape the run directory."""
    if not isinstance(path, str) or not path:
        return False
    candidate = pathlib.PurePosixPath(path)
    return not candidate.is_absolute() and ".." not in candidate.parts


def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _task_dirs(suite_dir):
    return sorted(path for path in suite_dir.iterdir()
                  if path.is_dir() and not path.name.startswith("."))


def validate_task(folder):
    """Problems in one task folder, as messages without the task prefix."""
    problems = []
    task_path = folder / "task.json"
    tests_path = folder / "tests.json"
    try:
        task = load_json(task_path)
    except FileNotFoundError:
        return ["missing task.json"]
    except (OSError, ValueError) as error:
        return [f"task.json: {error}"]
    if not isinstance(task, dict):
        return ["task.json: must be a JSON object"]

    for field in ("id", "category", "statement", "bash_natural", "files"):
        if field not in task:
            problems.append(f"missing field {field}")
    task_id = task.get("id")
    if not isinstance(task_id, str) or not task_id:
        problems.append("id must be a non-empty string")
    elif task_id != folder.name:
        problems.append(f"id {task_id!r} differs from the folder name")
    if task.get("category") not in CATEGORIES:
        problems.append(f"category must be one of {sorted(CATEGORIES)}")
    statement = task.get("statement")
    if not isinstance(statement, str) or not statement.strip():
        problems.append("statement must be a non-empty string")
    if not isinstance(task.get("bash_natural"), bool):
        problems.append("bash_natural must be true or false")
    limit = task.get("time_limit_s", DEFAULT_TIME_LIMIT)
    if not _is_number(limit) or limit <= 0:
        problems.append("time_limit_s must be a positive number")

    files = task.get("files")
    read_files, write_files = [], []
    if not isinstance(files, dict):
        problems.append("files must be an object with read and write lists")
    else:
        for key in ("read", "write"):
            if key not in files:
                problems.append(f"files.{key} is missing")
            elif not isinstance(files[key], list) or not all(isinstance(p, str) for p in files[key]):
                problems.append(f"files.{key} must be a list of strings")
            elif not all(_safe_rel(p) for p in files[key]):
                problems.append(f"files.{key} contains an unsafe path")
            else:
                (read_files if key == "read" else write_files).extend(files[key])

    try:
        tests = load_json(tests_path)
    except FileNotFoundError:
        return problems + ["missing tests.json"]
    except (OSError, ValueError) as error:
        return problems + [f"tests.json: {error}"]
    if not isinstance(tests, list):
        return problems + ["tests.json must be a list"]
    if len(tests) < 2:
        problems.append("tests.json must contain at least 2 tests")

    names = set()
    for index, test in enumerate(tests):
        where = f"test {index}"
        if not isinstance(test, dict):
            problems.append(f"{where}: must be an object")
            continue
        name = test.get("name")
        if not isinstance(name, str) or not name:
            problems.append(f"{where}: name must be a non-empty string")
        else:
            where = f"test {name}"
            if name in names:
                problems.append(f"{where}: duplicate test name")
            names.add(name)
        args = test.get("args")
        if not isinstance(args, list) or not all(isinstance(a, str) for a in args):
            problems.append(f"{where}: args must be a list of strings")
        if "stdin" in test and not isinstance(test["stdin"], str):
            problems.append(f"{where}: stdin must be a string")
        if not isinstance(test.get("stdout"), str):
            problems.append(f"{where}: stdout must be a string")
        code = test.get("exit_code", 0)
        if not isinstance(code, int) or isinstance(code, bool):
            problems.append(f"{where}: exit_code must be an integer")
        extra = test.get("files", {})
        if not isinstance(extra, dict) or not all(isinstance(k, str) and isinstance(v, str)
                                                  for k, v in extra.items()):
            problems.append(f"{where}: files must map paths to string contents")
            extra = {}
        elif not all(_safe_rel(path) for path in extra):
            problems.append(f"{where}: files contains an unsafe path")
        expected = test.get("expect_files", {})
        if not isinstance(expected, dict) or not all(isinstance(k, str) and isinstance(v, str)
                                                     for k, v in expected.items()):
            problems.append(f"{where}: expect_files must map paths to string contents")
        else:
            for path in expected:
                if not _safe_rel(path):
                    problems.append(f"{where}: expect_files contains an unsafe path")
                elif path not in write_files:
                    problems.append(f"{where}: expect_files path {path!r} is not in files.write")
        for path in read_files:
            if (folder / "inputs" / path).is_file():
                continue
            if path in extra:
                continue
            problems.append(f"{where}: declared input file {path!r} is missing from inputs/ and the test files")
    return problems


def validate_suite(suite_dir, refs=False, aleph=None):
    """One message per problem, each formatted as TASK: message."""
    suite_dir = pathlib.Path(suite_dir)
    problems = []
    seen = {}
    for folder in _task_dirs(suite_dir):
        found = validate_task(folder)
        try:
            task = load_json(folder / "task.json")
        except (OSError, ValueError):
            task = None
        if isinstance(task, dict) and isinstance(task.get("id"), str):
            task_id = task["id"]
            if task_id in seen:
                found.append(f"duplicate id {task_id!r} (also in {seen[task_id]})")
            else:
                seen[task_id] = folder.name
        problems.extend(f"{folder.name}: {message}" for message in found)

    if refs:
        for folder in _task_dirs(suite_dir):
            ref_dir = folder / "ref"
            if not ref_dir.is_dir():
                continue
            for path in sorted(ref_dir.iterdir()):
                lang = REF_LANG.get(path.suffix)
                if not lang or not path.is_file():
                    continue
                try:
                    results = run_tests(folder, lang, path, aleph=aleph)
                except (OSError, ValueError, KeyError) as error:
                    problems.append(f"{folder.name}: {path.name}: cannot run: {error}")
                    continue
                for result in results:
                    if not result["passed"]:
                        problems.append(
                            f"{folder.name}: {path.name}: {result['name']}: {result['detail']}")
    return problems


def _command(lang, program, args, aleph):
    if lang == "python":
        return ["python3", str(program)] + list(args)
    if lang == "bash":
        return ["bash", str(program)] + list(args)
    if lang == "aleph":
        return [aleph or default_aleph(), "run", str(program)] + list(args)
    raise ValueError(f"unknown language {lang!r}; expected one of {sorted(LANGS)}")


def _write_file(base, rel, content):
    if not _safe_rel(rel):
        raise ValueError(f"unsafe relative path: {rel!r}")
    path = base / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def _run_one(task_dir, lang, program, aleph, test, limit, log_access=False):
    name = test.get("name", "?")
    with tempfile.TemporaryDirectory() as directory:
        work = pathlib.Path(directory)
        inputs = task_dir / "inputs"
        if inputs.is_dir():
            shutil.copytree(inputs, work, dirs_exist_ok=True)
        for rel, content in test.get("files", {}).items():
            _write_file(work, rel, content)

        log_fields = {}
        logged = log_access and lang in ("python", "bash")
        if logged:
            sandbox = _sandbox_module()
            try:
                proc = sandbox.run_logged(lang, program, test.get("args", []), work,
                                          test.get("stdin"), limit)
            except subprocess.TimeoutExpired:
                return {"name": name, "passed": False, "detail": f"timeout after {limit}s"}
            except OSError as error:
                return {"name": name, "passed": False, "detail": f"cannot run: {error}"}
            task = load_json(task_dir / "task.json")
            files = task.get("files", {})
            log_fields = {
                "accesses": proc["accesses"],
                "outside": sandbox.outside(proc["accesses"], files.get("read", []),
                                           files.get("write", []), work),
                "observed": proc["observed"],
            }
            stdout_bytes = proc["stdout"].encode("utf-8")
            stderr = proc["stderr"]
            returncode = proc["returncode"]
        else:
            command = _command(lang, program, test.get("args", []), aleph)
            kwargs = {"cwd": work, "capture_output": True}
            if "stdin" in test:
                kwargs["input"] = test["stdin"].encode("utf-8")
            else:
                kwargs["stdin"] = subprocess.DEVNULL
            try:
                proc = subprocess.run(command, timeout=limit, **kwargs)
            except subprocess.TimeoutExpired:
                return {"name": name, "passed": False, "detail": f"timeout after {limit}s"}
            except OSError as error:
                return {"name": name, "passed": False, "detail": f"cannot run: {error}"}
            stdout_bytes = proc.stdout
            stderr = proc.stderr.decode("utf-8", errors="replace")
            returncode = proc.returncode

        expected_stdout = test.get("stdout", "")
        actual_stdout = stdout_bytes.decode("utf-8", errors="replace")
        if stdout_bytes != expected_stdout.encode("utf-8"):
            return dict({"name": name, "passed": False,
                         "detail": f"stdout: expected {expected_stdout!r}, "
                                   f"got {actual_stdout!r}"}, **log_fields)
        expected_code = test.get("exit_code", 0)
        if returncode != expected_code:
            return dict({"name": name, "passed": False,
                         "detail": f"exit code: expected {expected_code}, got {returncode}"
                                   + (f"; stderr {stderr!r}" if stderr else "")}, **log_fields)
        for rel, expected in test.get("expect_files", {}).items():
            path = work / rel
            if not path.is_file():
                return dict({"name": name, "passed": False,
                             "detail": f"missing output file {rel!r}"}, **log_fields)
            actual = path.read_bytes()
            if actual != expected.encode("utf-8"):
                return dict({"name": name, "passed": False,
                             "detail": f"file {rel!r}: expected {expected!r}, "
                                       f"got {actual.decode('utf-8', errors='replace')!r}"},
                            **log_fields)
        return dict({"name": name, "passed": True, "detail": "ok"}, **log_fields)


def run_tests(task_dir, lang, program_path, aleph=None, log_access=False):
    """Run every hidden test; return [{name, passed, detail, ...}].

    With log_access=True, python and bash results also carry accesses, outside
    and observed from the sandbox.
    """
    task_dir = pathlib.Path(task_dir)
    task = load_json(task_dir / "task.json")
    tests = load_json(task_dir / "tests.json")
    limit = task.get("time_limit_s", DEFAULT_TIME_LIMIT)
    program = pathlib.Path(program_path).resolve()
    return [_run_one(task_dir, lang, program, aleph, test, limit, log_access=log_access)
            for test in tests]


def _cmd_validate(args):
    problems = validate_suite(args.suite_dir, refs=args.refs)
    for problem in problems:
        print(problem)
    return 1 if problems else 0


def _cmd_run(args):
    results = run_tests(args.task_dir, args.lang, args.program_file,
                        log_access=args.log_access)
    if args.json:
        print(json.dumps(results, indent=2))
    else:
        for result in results:
            print(f"{'PASS' if result['passed'] else 'FAIL'} {result['name']}: {result['detail']}")
    return 0 if all(result["passed"] for result in results) else 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate", help="check task folders")
    validate.add_argument("suite_dir")
    validate.add_argument("--refs", action="store_true", help="also run reference solutions")
    validate.set_defaults(func=_cmd_validate)
    run = sub.add_parser("run", help="run one program against a task")
    run.add_argument("task_dir")
    run.add_argument("lang", choices=sorted(LANGS))
    run.add_argument("program_file")
    run.add_argument("--json", action="store_true")
    run.add_argument("--log-access", action="store_true",
                     help="log file accesses for python and bash programs")
    run.set_defaults(func=_cmd_run)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
