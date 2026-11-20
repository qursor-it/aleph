"""Offline tests for suite.py: python3 -m unittest -v test_suite."""
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import suite  # noqa: E402

HERE = pathlib.Path(__file__).resolve().parent
SAMPLES = HERE.parents[1] / "tasks" / "EXP-002-samples"


def aleph_binary():
    candidate = os.environ.get("ALEPH")
    if candidate and pathlib.Path(candidate).is_file():
        return str(pathlib.Path(candidate).resolve())
    return shutil.which("aleph")


ALEPH = aleph_binary()


def make_test(name):
    return {"name": name, "args": [], "stdout": "hello\n", "exit_code": 0}


def make_task(root, folder_name, task=None, tests=None):
    folder = pathlib.Path(root) / folder_name
    folder.mkdir()
    data = {
        "id": folder_name,
        "category": "text",
        "statement": "Print hello.",
        "bash_natural": False,
        "files": {"read": [], "write": []},
        "time_limit_s": 10,
    }
    data.update(task or {})
    (folder / "task.json").write_text(json.dumps(data), encoding="utf-8")
    entries = tests if tests is not None else [make_test("one"), make_test("two")]
    (folder / "tests.json").write_text(json.dumps(entries), encoding="utf-8")
    return folder


class ValidatorTest(unittest.TestCase):
    def test_samples_are_valid(self):
        self.assertEqual(suite.validate_suite(SAMPLES), [])

    def test_cli_accepts_samples(self):
        proc = subprocess.run([sys.executable, str(HERE / "suite.py"), "validate", str(SAMPLES)],
                              capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_missing_field(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = make_task(directory, "missing")
            task = json.loads((folder / "task.json").read_text(encoding="utf-8"))
            del task["statement"]
            (folder / "task.json").write_text(json.dumps(task), encoding="utf-8")
            problems = suite.validate_suite(directory)
        self.assertTrue(any("missing field statement" in problem for problem in problems), problems)

    def test_duplicate_id(self):
        with tempfile.TemporaryDirectory() as directory:
            make_task(directory, "a", task={"id": "dup"})
            make_task(directory, "b", task={"id": "dup"})
            problems = suite.validate_suite(directory)
        self.assertTrue(any("duplicate id" in problem for problem in problems), problems)

    def test_undeclared_write_path(self):
        with tempfile.TemporaryDirectory() as directory:
            make_task(directory, "undeclared", tests=[
                dict(make_test("one"), expect_files={"other.txt": "x"}),
                make_test("two"),
            ])
            problems = suite.validate_suite(directory)
        self.assertTrue(any("expect_files path 'other.txt'" in problem for problem in problems),
                        problems)

    def test_missing_input_file(self):
        with tempfile.TemporaryDirectory() as directory:
            make_task(directory, "noinput", task={"files": {"read": ["in.txt"], "write": []}})
            problems = suite.validate_suite(directory)
        self.assertTrue(any("missing from inputs/" in problem for problem in problems), problems)

    def test_single_test(self):
        with tempfile.TemporaryDirectory() as directory:
            make_task(directory, "single", tests=[make_test("only")])
            problems = suite.validate_suite(directory)
        self.assertTrue(any("at least 2 tests" in problem for problem in problems), problems)


class RunnerTest(unittest.TestCase):
    def test_python_reference_solutions_pass(self):
        for folder in sorted(SAMPLES.iterdir()):
            solution = folder / "ref" / "solution.py"
            if not solution.is_file():
                continue
            results = suite.run_tests(folder, "python", solution)
            self.assertTrue(all(result["passed"] for result in results),
                            (folder.name, results))

    def test_bash_reference_solution_passes(self):
        folder = SAMPLES / "files-copy-prefix"
        results = suite.run_tests(folder, "bash", folder / "ref" / "solution.sh")
        self.assertTrue(all(result["passed"] for result in results), results)

    def test_wrong_program_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            program = pathlib.Path(directory) / "wrong.py"
            program.write_text("print('wrong')\n", encoding="utf-8")
            results = suite.run_tests(SAMPLES / "algorithm-primes", "python", program)
        self.assertTrue(any(not result["passed"] for result in results), results)

    def test_time_limit_is_enforced(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = make_task(directory, "slow", task={"time_limit_s": 0.2})
            program = pathlib.Path(directory) / "slow.py"
            program.write_text("import time\ntime.sleep(2)\n", encoding="utf-8")
            results = suite.run_tests(folder, "python", program)
        self.assertTrue(results)
        self.assertTrue(all(not result["passed"] for result in results), results)
        self.assertTrue(all("timeout" in result["detail"] for result in results), results)

    @unittest.skipUnless(ALEPH, "aleph binary not found; set ALEPH")
    def test_aleph_reference_solutions_pass(self):
        for folder in sorted(SAMPLES.iterdir()):
            solution = folder / "ref" / "solution.aph"
            if not solution.is_file():
                continue
            results = suite.run_tests(folder, "aleph", solution, aleph=ALEPH)
            self.assertTrue(all(result["passed"] for result in results),
                            (folder.name, results))


if __name__ == "__main__":
    unittest.main()
