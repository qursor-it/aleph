"""Offline tests: python3 -m unittest -v test_sandbox."""
import os
import pathlib
import shutil
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import sandbox  # noqa: E402

SUITE_DIR = HERE.parent / "suite"
sys.path.insert(0, str(SUITE_DIR))
import suite  # noqa: E402

SAMPLES = HERE.parents[1] / "tasks" / "EXP-002-samples"
BASH = shutil.which("bash")
STRACE = sandbox.strace_path()


def write_program(cwd, source, name="program.py"):
    path = pathlib.Path(cwd) / name
    path.write_text(source, encoding="utf-8")
    return path


def run_python(source, cwd):
    program = write_program(cwd, source)
    return sandbox.run_logged("python", program, [], cwd, None, 30)


def paths(accesses, op=None):
    return [access["path"] for access in accesses
            if op is None or access.get("op") == op]


class PythonLoggingTest(unittest.TestCase):
    def test_declared_read_and_undeclared_write(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            (cwd / "in.txt").write_text("hello\n", encoding="utf-8")
            result = run_python(
                "data = open('in.txt').read()\n"
                "open('out.txt', 'w').write(data.upper())\n",
                cwd)
            self.assertTrue(result["observed"])
            self.assertEqual(result["returncode"], 0)
            self.assertIn(str(cwd / "in.txt"), paths(result["accesses"], "read"))
            self.assertIn(str(cwd / "out.txt"), paths(result["accesses"], "write"))
            outside = sandbox.outside(result["accesses"], ["in.txt"], [], cwd)
            self.assertIn(str(cwd / "out.txt"), paths(outside, "write"))
            self.assertNotIn(str(cwd / "in.txt"), paths(outside))

    def test_read_outside_run_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            result = run_python(
                "try:\n"
                "    open('/etc/hostname').read()\n"
                "except OSError:\n"
                "    pass\n",
                cwd)
            self.assertEqual(result["returncode"], 0)
            self.assertIn("/etc/hostname", paths(result["accesses"]))
            outside = sandbox.outside(result["accesses"], [], [], cwd)
            self.assertIn("/etc/hostname", paths(outside))

    def test_interpreter_startup_is_not_logged(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            result = run_python(
                "import json\nprint(json.dumps({'ok': True}))\n", cwd)
            self.assertEqual(result["stdout"], '{"ok": true}\n')
            self.assertEqual(result["accesses"], [])

    def test_declared_write_file_may_be_read(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            access = {"op": "read", "path": str(cwd / "out.txt"), "ok": True}
            self.assertEqual(
                sandbox.outside([access], ["in.txt"], ["out.txt"], cwd), [])

    def test_writing_declared_read_file_is_outside(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            access = {"op": "write", "path": str(cwd / "in.txt"), "ok": True}
            outside = sandbox.outside([access], ["in.txt"], [], cwd)
            self.assertEqual(paths(outside), [str(cwd / "in.txt")])


@unittest.skipUnless(STRACE, "strace not installed")
class BashLoggingTest(unittest.TestCase):
    def test_cat_redirect_is_read_and_write(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            (cwd / "in.txt").write_text("hello\n", encoding="utf-8")
            program = write_program(cwd, "cat in.txt > out.txt\n", "program.sh")
            result = sandbox.run_logged("bash", program, [], cwd, None, 30)
            self.assertTrue(result["observed"])
            self.assertEqual(result["returncode"], 0)
            ops = {(access["op"], access["path"]) for access in result["accesses"]}
            self.assertIn(("read", str(cwd / "in.txt")), ops)
            self.assertIn(("write", str(cwd / "out.txt")), ops)
            outside = sandbox.outside(
                result["accesses"], ["in.txt"], ["out.txt"], cwd)
            self.assertEqual(outside, [])

    def test_failed_access_is_recorded_as_not_ok(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            program = write_program(cwd, "cat missing.txt > out.txt\n", "program.sh")
            result = sandbox.run_logged("bash", program, [], cwd, None, 30)
            failed = [access for access in result["accesses"]
                      if access["path"] == str(cwd / "missing.txt")]
            self.assertTrue(failed)
            self.assertFalse(failed[0]["ok"])


@unittest.skipUnless(BASH, "bash not installed")
class BashWithoutStraceTest(unittest.TestCase):
    def test_not_observed_when_strace_is_missing(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = pathlib.Path(directory)
            bin_dir = cwd / "bin"
            bin_dir.mkdir()
            os.symlink(BASH, bin_dir / "bash")
            program = write_program(cwd, "echo hi\n", "program.sh")
            previous = os.environ.get("PATH")
            os.environ["PATH"] = str(bin_dir)
            try:
                result = sandbox.run_logged("bash", program, [], cwd, None, 30)
            finally:
                if previous is None:
                    os.environ.pop("PATH", None)
                else:
                    os.environ["PATH"] = previous
            self.assertFalse(result["observed"])
            self.assertEqual(result["accesses"], [])
            self.assertEqual(result["returncode"], 0)
            self.assertEqual(result["stdout"], "hi\n")


class SuiteIntegrationTest(unittest.TestCase):
    def test_run_tests_logs_accesses(self):
        folder = SAMPLES / "files-copy-prefix"
        solution = folder / "ref" / "solution.py"
        results = suite.run_tests(folder, "python", solution, log_access=True)
        self.assertTrue(results)
        self.assertTrue(all(result["passed"] for result in results), results)
        for result in results:
            self.assertIn("accesses", result)
            self.assertIn("outside", result)
            self.assertTrue(result["observed"])
            self.assertEqual(result["outside"], [], result)
        ops = {(access["op"], os.path.basename(access["path"]))
               for result in results for access in result["accesses"]}
        self.assertIn(("read", "in.txt"), ops)
        self.assertIn(("write", "out.txt"), ops)

    def test_default_run_has_no_access_fields(self):
        folder = SAMPLES / "algorithm-primes"
        solution = folder / "ref" / "solution.py"
        results = suite.run_tests(folder, "python", solution)
        self.assertNotIn("accesses", results[0])
        self.assertNotIn("observed", results[0])


if __name__ == "__main__":
    unittest.main()
