"""Offline tests for reward.py. Aleph cases skip without the aleph binary."""
import os
import pathlib
import sys
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import reward as rw  # noqa: E402

REPO = HERE.parents[2]
TASK = REPO / "experiments" / "tasks" / "EXP-002-samples" / "algorithm-primes"
ALEPH = os.environ.get("ALEPH") or str(REPO / "target" / "debug" / "aleph")
have_aleph = pathlib.Path(ALEPH).is_file()

REF_APH = (TASK / "ref" / "solution.aph").read_text()
REF_PY = (TASK / "ref" / "solution.py").read_text()
# Prints nothing: passes only the two tests whose expected output is empty.
HALF_APH = "fn main -> res unit str\n ok nil\n"
HELPER = "fn one -> i64\n{ex}\n 1\n"
ONE_EX = " ex one = 1"
EXTRA_EX = "\n".join([ONE_EX] * 4)
TYPE_ERROR = 'fn main -> res unit str\n let x + 1 "a"\n ok nil\n'
CAP_TRAP = ('use fs.read "data"\nfn main -> res unit str\n let p cat "ot" "her.txt"\n'
            ' let t try read p\n ok nil\n')
# ~5e9 additions: runs far beyond a 1 s limit.
SLOW = """fn inner a:i64 j:i64 -> i64
 ex inner 1 2 = 3
 + a j
fn outer a:i64 i:i64 -> i64
 ex outer 1 2 = 4999950001
 + a fold @inner 0 range 0 100000
fn main -> res unit str
 let x fold @outer 0 range 0 1000000
 ok nil
"""


def score(text, lang="aleph", **kw):
    return rw.reward(text, TASK, lang=lang, aleph=ALEPH, **kw)


@unittest.skipUnless(have_aleph, "aleph binary not found (set ALEPH)")
class AlephReward(unittest.TestCase):
    def test_reference_is_one(self):
        r = score(REF_APH)
        self.assertEqual((r["reward"], r["tests_passed"], r["tests_total"]), (1.0, 4, 4))
        self.assertTrue(r["check_ok"])

    def test_half_the_tests(self):
        r = score(HALF_APH)
        self.assertEqual(r["reward"], 0.5)
        self.assertTrue(r["check_ok"])

    def test_type_error_is_zero(self):
        r = score(TYPE_ERROR)
        self.assertEqual(r["reward"], 0.0)
        self.assertIs(r["check_ok"], False)
        self.assertIn("T001", r["detail"])

    def test_capability_trap_is_zero(self):
        r = score(CAP_TRAP)
        self.assertEqual(r["reward"], 0.0)
        self.assertTrue(r["check_ok"])
        self.assertTrue(r["capability_violation"])

    def test_infinite_loop_times_out(self):
        r = score(SLOW, time_limit=1)
        self.assertEqual(r["reward"], 0.0)
        self.assertTrue(r["timed_out"])

    def test_extra_examples_add_nothing(self):
        few = score(HELPER.format(ex=ONE_EX) + HALF_APH)
        many = score(HELPER.format(ex=EXTRA_EX) + HALF_APH)
        self.assertEqual(few["reward"], many["reward"])
        self.assertEqual(few["reward"], 0.5)


class PythonReward(unittest.TestCase):
    def test_reference_is_one(self):
        self.assertEqual(score(REF_PY, "python")["reward"], 1.0)

    def test_half_the_tests(self):
        r = score("pass\n", "python")
        self.assertEqual(r["reward"], 0.5)

    def test_outside_access_is_zero(self):
        r = score(REF_PY + "\nopen('leak.txt', 'w').write('x')\n", "python")
        self.assertEqual(r["tests_passed"], 4)
        self.assertEqual(r["reward"], 0.0)
        self.assertTrue(r["capability_violation"])

    def test_infinite_loop_times_out(self):
        r = score("while True:\n    pass\n", "python", time_limit=0.5)
        self.assertEqual(r["reward"], 0.0)
        self.assertTrue(r["timed_out"])


class Batch(unittest.TestCase):
    def test_reward_many_and_task_untouched(self):
        before = sorted(p.name for p in TASK.rglob("*"))
        out = rw.reward_many([{"program_text": REF_PY, "task_dir": TASK, "lang": "python"},
                              {"program_text": "pass", "task_dir": TASK / "missing", "lang": "python"}])
        self.assertEqual(out[0]["reward"], 1.0)
        self.assertEqual(out[1]["reward"], 0.0)
        self.assertTrue(out[1]["detail"].startswith("error"))
        self.assertEqual(before, sorted(p.name for p in TASK.rglob("*")))


if __name__ == "__main__":
    unittest.main()
