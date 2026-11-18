"""Offline tests: python3 -m unittest experiments/benchmarks/humaneval/test_humaneval_to_aleph.py"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import humaneval_to_aleph as t  # noqa: E402

PROBLEM = {
    "task_id": "HumanEval/999",
    "entry_point": "sum_pair",
    "prompt": '''from typing import List, Tuple, Optional, Dict


def sum_pair(numbers: List[float], label: str) -> Tuple[float, str]:
    """ Add up numbers and keep the label.
    >>> sum_pair([1, 2.5], 'a')
    (3.5, 'a')
    """
''',
    "test": '''
def check(candidate):
    assert candidate([], 'x') == (0.0, 'x')
    assert candidate([1e-7], "q\\"") == (1e-7, 'q"')
    assert abs(candidate([1.0], 'z')[0] - 1.0) < 1e-6
''',
}


class TranslateTest(unittest.TestCase):
    def test_identifiers_drop_underscores_and_avoid_reserved_names(self):
        self.assertEqual(t.ident("has_close_elements"), "hascloseelements")
        self.assertEqual(t.ident("sort"), "sortx")
        self.assertEqual(t.ident("_x1"), "x1")

    def test_float_literals_have_a_point_and_no_exponent(self):
        self.assertEqual(t.float_literal(3), "3.0")
        self.assertEqual(t.float_literal(1e-7), "0.0000001")
        self.assertEqual(t.float_literal(-0.0), "0.0")
        with self.assertRaises(t.Skip):
            t.float_literal(float("nan"))

    def test_problem_with_tuple_result(self):
        record = t.translate(PROBLEM)
        self.assertEqual(record["name"], "HumanEval_999_sum_pair")
        self.assertIn("type tup0 a:f64 b:str\n", record["prompt"])
        self.assertIn("fn sumpair numbers:list f64 label:str -> tup0\n", record["prompt"])
        self.assertIn(' ex sumpair [1.0 2.5] "a" = tup0 3.5 "a"', record["prompt"])
        self.assertIn(' ex sumpair empty f64 "x" = tup0 0.0 "x"', record["tests"])
        self.assertIn(' ex sumpair [0.0000001] "q\\"" = tup0 0.0000001 "q\\""', record["tests"])
        self.assertEqual(record["tests_total"], 3)
        self.assertEqual(record["tests_translated"], 2)

    def test_optional_and_dict_values(self):
        types = t.Types()
        values = t.Values(types)
        self.assertEqual(values.expr(None, ("opt", ("i64",))), "none i64")
        self.assertEqual(values.expr(5, ("opt", ("i64",))), "some 5")
        self.assertEqual(values.expr({"a": 1}, ("dict", ("str",), ("i64",))), 'put dict str i64 "a" 1')

    def test_untranslatable_types_are_skipped(self):
        problem = dict(PROBLEM, prompt=PROBLEM["prompt"].replace("label: str", "label: Any"))
        with self.assertRaises(t.Skip):
            t.translate(problem)


if __name__ == "__main__":
    unittest.main()
