#!/usr/bin/env python3
"""Offline tests for the EXP-003 corpus tool."""
import json
import os
import pathlib
import tempfile
import unittest

import corpus


def _aleph():
    path = os.environ.get("ALEPH")
    if path and pathlib.Path(path).is_file():
        return path
    default = pathlib.Path(corpus.DEFAULT_ALEPH)
    if default.is_file():
        return str(default)
    return None


def _write_program(root, name, meta, tests, files):
    program = pathlib.Path(root) / name
    program.mkdir(parents=True)
    (program / "meta.json").write_text(json.dumps(meta), encoding="utf-8")
    (program / "tests.json").write_text(json.dumps(tests), encoding="utf-8")
    for filename, content in files.items():
        (program / filename).write_text(content, encoding="utf-8")
    return program


class NormaliserTests(unittest.TestCase):
    def test_py_norm_removes_docstrings_and_comments(self):
        source = (
            '"""module docstring"""\n'
            "# a comment\n"
            "import os\n\n"
            "class Thing:\n"
            '    """class docstring"""\n'
            "    def method(self):\n"
            '        """method docstring"""\n'
            "        return 1\n"
        )
        normalised = corpus.py_norm(source)
        self.assertNotIn('"""', normalised)
        self.assertNotIn("# a comment", normalised)
        self.assertIn("import os", normalised)
        self.assertIn("def method", normalised)
        self.assertIn("return 1", normalised)
        self.assertTrue(normalised.endswith("\n"))

    def test_py_norm_keeps_pass_for_docstring_only_body(self):
        normalised = corpus.py_norm('def f():\n    """only a docstring"""\n')
        self.assertIn("pass", normalised)

    def test_strip_c_comments_keeps_literals_and_lines(self):
        source = (
            "// line comment\n"
            "int x = 1; /* block\ncomment */\n"
            'char *s = "a//b";\n'
            "char c = '/';\n"
        )
        stripped = corpus.strip_c_comments(source)
        self.assertNotIn("line comment", stripped)
        self.assertNotIn("block", stripped)
        self.assertIn("int x = 1;", stripped)
        self.assertIn('"a//b"', stripped)
        self.assertIn("'/'", stripped)
        self.assertGreaterEqual(stripped.count("\n"), 4)

    def test_strip_bash_comments_respects_quotes_and_expansions(self):
        source = (
            "#!/usr/bin/env bash\n"
            "# a comment\n"
            'echo "a # b" # tail\n'
            "echo '# c'\n"
            "x=${y#z}\n"
        )
        stripped = corpus.strip_bash_comments(source)
        self.assertNotIn("a comment", stripped)
        self.assertNotIn("tail", stripped)
        self.assertIn('"a # b"', stripped)
        self.assertIn("'# c'", stripped)
        self.assertIn("${y#z}", stripped)

    def test_remove_examples(self):
        source = "fn f x:i64 -> i64\n ex f 1 = 1\n x\n"
        logic = corpus.remove_examples(source)
        self.assertNotIn(" ex ", logic)
        self.assertIn("fn f", logic)
        self.assertIn(" x", logic)


class StatisticsTests(unittest.TestCase):
    def test_wilcoxon_exact_known_examples(self):
        result = corpus.wilcoxon([1, 2, 3, 4, 5])
        self.assertEqual(result["n"], 5)
        self.assertEqual(result["statistic"], 15)
        self.assertEqual(result["method"], "exact")
        self.assertAlmostEqual(result["p"], 2 / 32)

        result = corpus.wilcoxon([1, 2, 3, 4, 5, 6])
        self.assertAlmostEqual(result["p"], 2 / 64)

    def test_wilcoxon_drops_zero_differences(self):
        result = corpus.wilcoxon([0, 0, 0])
        self.assertEqual(result["n"], 0)
        self.assertEqual(result["p"], 1.0)

    def test_wilcoxon_ties(self):
        result = corpus.wilcoxon([1, 1, 1, 1])
        self.assertEqual(result["n"], 4)
        self.assertAlmostEqual(result["statistic"], 10)
        self.assertAlmostEqual(result["p"], 2 / 16)

    def test_wilcoxon_normal_approximation_for_large_n(self):
        result = corpus.wilcoxon(list(range(1, 26)))
        self.assertEqual(result["method"], "normal")
        self.assertEqual(result["n"], 25)
        self.assertGreaterEqual(result["p"], 0.0)
        self.assertLessEqual(result["p"], 1.0)
        self.assertLess(result["p"], 0.001)

    def test_bootstrap_is_seeded_and_brackets_median(self):
        values = [1.0, 2.0, 3.0, 4.0, 5.0]
        first = corpus.bootstrap_median_ci(values, seed=7, iterations=500)
        second = corpus.bootstrap_median_ci(values, seed=7, iterations=500)
        self.assertEqual(first, second)
        self.assertLessEqual(first[0], 3.0)
        self.assertGreaterEqual(first[1], 3.0)

    def test_break_even(self):
        self.assertEqual(corpus.break_even(100, 30, 10), 5.0)
        self.assertIsNone(corpus.break_even(100, 10, 10))
        self.assertIsNone(corpus.break_even(100, 10, 20))
        self.assertIsNone(corpus.break_even(None, 30, 10))


class CorpusTests(unittest.TestCase):
    def _tiny_corpus(self, root, with_aleph):
        tests = [
            {"name": "default", "args": [], "stdout": "hi\n", "exit_code": 0},
            {"name": "argument", "args": ["x"], "stdout": "x\n", "exit_code": 0},
        ]
        files = {
            "python.py": (
                "import sys\n"
                'print(sys.argv[1] if len(sys.argv) > 1 else "hi")\n'
            )
        }
        if with_aleph:
            files["aleph.aph"] = (
                "fn main -> unit\n"
                ' let x else at args 0 "hi"\n'
                " print x\n"
            )
        return _write_program(
            root,
            "tiny",
            {
                "program": "tiny",
                "group": "bench",
                "category": "text",
                "source": {},
            },
            tests,
            files,
        )

    def test_verify_tiny_corpus(self):
        aleph = _aleph()
        with tempfile.TemporaryDirectory() as directory:
            self._tiny_corpus(directory, with_aleph=aleph is not None)
            failures = corpus.verify(directory, aleph=aleph or "aleph")
        self.assertEqual(failures, [])

    def test_verify_reports_a_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            program = self._tiny_corpus(directory, with_aleph=False)
            (program / "python.py").write_text(
                "print('wrong')\n", encoding="utf-8"
            )
            failures = corpus.verify(directory, aleph="aleph")
        self.assertTrue(failures)
        self.assertIn("tiny: python", failures[0])

    def test_analysis_tokenizers_prefers_bpe(self):
        programs = [
            {
                "tokens": {
                    "aleph": {
                        "chars4": {"full": 1, "logic": 1},
                        "bpe:demo": {"full": 1, "logic": 1},
                    }
                }
            }
        ]
        self.assertEqual(corpus.analysis_tokenizers(programs), ["bpe:demo"])

    def test_analysis_tokenizers_falls_back_to_chars4(self):
        programs = [
            {"tokens": {"aleph": {"chars4": {"full": 1, "logic": 1}}}}
        ]
        self.assertEqual(corpus.analysis_tokenizers(programs), ["chars4"])

    def test_compute_report_shape(self):
        programs = [
            {
                "program": "a",
                "group": "bench",
                "category": "algorithm",
                "source": {},
                "languages": ["aleph", "python"],
                "formatters": {"aleph": "aleph fmt", "python": "ast.unparse"},
                "tokens": {
                    "aleph": {"chars4": {"full": 10, "logic": 8}},
                    "python": {"chars4": {"full": 20, "logic": 20}},
                },
            },
            {
                "program": "b",
                "group": "bench",
                "category": "algorithm",
                "source": {},
                "languages": ["aleph", "python"],
                "formatters": {"aleph": "aleph fmt", "python": "ast.unparse"},
                "tokens": {
                    "aleph": {"chars4": {"full": 30, "logic": 22}},
                    "python": {"chars4": {"full": 25, "logic": 25}},
                },
            },
        ]
        report = corpus.compute_report(
            programs, ["chars4"], {"chars4": 100}, seed=0, iterations=50
        )
        self.assertEqual(
            report["tokenizers"], [{"name": "chars4", "estimate": True}]
        )
        self.assertEqual(report["coverage"]["programs"], 2)
        self.assertEqual(report["coverage"]["aleph"], 2)
        self.assertEqual(report["coverage"]["c"], 0)
        self.assertEqual(report["ratios"]["chars4"]["without_examples"]["n"], 2)
        self.assertEqual(report["ratios"]["chars4"]["with_examples"]["n"], 2)
        self.assertIn("bench", report["groups"]["chars4"])
        self.assertIn("algorithm", report["categories"]["chars4"])
        # 100 / ((20 + 25) - (8 + 22)) = 100 / 15
        self.assertAlmostEqual(
            report["break_even"]["chars4"]["bench"], 100 / 15
        )
        self.assertEqual(report["llms_tokens"]["chars4"], 100)
        json.dumps(report)

    @unittest.skipUnless(_aleph() is not None, "aleph binary not available")
    def test_build_report_tiny_corpus(self):
        aleph = _aleph()
        with tempfile.TemporaryDirectory() as directory:
            self._tiny_corpus(directory, with_aleph=True)
            report = corpus.build_report(
                directory, seed=0, aleph=aleph, iterations=50
            )
        self.assertEqual(report["coverage"]["aleph"], 1)
        self.assertEqual(report["coverage"]["python"], 1)
        self.assertEqual(
            [entry["name"] for entry in report["tokenizers"]], ["chars4"]
        )
        self.assertEqual(report["programs"][0]["formatters"]["aleph"],
                         "aleph fmt")
        self.assertIn("chars4", report["ratios"])
        self.assertIn("chars4", report["break_even"])
        json.dumps(report)


if __name__ == "__main__":
    unittest.main()
