#!/usr/bin/env python3
"""Offline tests for humaneval_ab.py: fake model only, no network."""
import gzip
import json
import pathlib
import tempfile
import unittest

import humaneval_ab as h

BODY = " let s sort numbers\n > len filter lam i < - nth s + i 1 nth s i threshold range 0 - len s 1 0\n"


def block(code, lang):
    return f"```{lang}\n{code}```"


class HumanEvalABTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prob = h.problems()[0]
        canon = next(map(json.loads, gzip.open(h.HE / "python" / "HumanEval.jsonl.gz", "rt")))
        cls.py_good = cls.prob["python"]["prompt"] + canon["canonical_solution"]
        cls.al_good = cls.prob["aleph"]["prompt"] + BODY

    def test_problem_set_pairs_every_aleph_problem_with_python(self):
        probs = h.problems()
        self.assertEqual(len(probs), 156)
        self.assertTrue(all(p["aleph"]["original"] == p["python"]["task_id"] for p in probs))

    def test_checks_and_hidden_scores(self):
        self.assertEqual(h.visible_check("aleph", self.al_good, self.prob), (True, ""))
        self.assertEqual(h.hidden_score("aleph", self.al_good, self.prob), "pass")
        self.assertTrue(h.visible_check("python", self.py_good, self.prob)[0])
        self.assertEqual(h.hidden_score("python", self.py_good, self.prob), "pass")
        wrong_py = self.prob["python"]["prompt"] + "    return True\n"
        ok, out = h.visible_check("python", wrong_py, self.prob)
        self.assertFalse(ok)
        self.assertIn("example failed", out)
        self.assertEqual(h.hidden_score("python", wrong_py, self.prob), "wrong")
        self.assertEqual(h.hidden_score("aleph", "fn other -> i64\n 1\n", self.prob), "no-compile")
        self.assertEqual(h.hidden_score("python", "def (", self.prob), "no-compile")

    def test_round_loop_feeds_back_and_stops_at_visible_pass(self):
        wrong = self.prob["aleph"]["prompt"] + " true\n"
        with tempfile.TemporaryDirectory() as tmp:
            answers = pathlib.Path(tmp, "a.json")
            answers.write_text(json.dumps([block(wrong, "aleph"), block(self.al_good, "aleph"), "unused"]))
            backend = h.FakeBackend(str(answers))
            rounds = h.session(backend, "aleph", self.prob, 5)
        self.assertEqual([r["visible_ok"] for r in rounds], [False, True])
        self.assertEqual([r["hidden"] for r in rounds], ["wrong", "pass"])
        self.assertIn("E002", rounds[0]["check"])

    def test_backend_failure_raises_and_keeps_the_prior_session(self):
        b = h.OpencodeBackend.__new__(h.OpencodeBackend)
        b.session = "good"
        calls = []

        def broken(message):
            calls.append(message)
            b.session = "broken"
            return "", {"error": "Unexpected server error"}
        b._send_once = broken
        sleep, h.time.sleep = h.time.sleep, lambda s: None
        try:
            with self.assertRaises(h.BackendError):
                b.send("hi")
        finally:
            h.time.sleep = sleep
        self.assertEqual((len(calls), b.session), (5, "good"))

    def test_result_metrics_gap_and_ratio(self):
        def rec(name, lang, first, tokens):
            return {"name": name, "lang": lang, "first": first, "final": "pass", "rounds": [
                {"usage": {"input": tokens, "output": 0, "steps": 1}, "wall_s": 1.0, "code": ""}]}
        recs = [rec("a", "python", "pass", 100), rec("a", "aleph", "wrong", 300),
                rec("b", "python", "pass", 100), rec("b", "aleph", "pass", 300)]
        m = {(x["name"], x.get("condition")): x for x in h.result_metrics(recs, overhead=0, resamples=50)}
        self.assertEqual(m[("pass@1 gap aleph-python", None)]["value"], -0.5)
        self.assertEqual(m[("tokens_per_solved ratio aleph/python", None)]["value"], 3.0)
        self.assertEqual(m[("tokens_per_solved ratio aleph/python", None)]["ci95"], [3.0, 3.0])

    def test_summary_counts_pass_rounds_and_net_tokens(self):
        rec = {"lang": "aleph", "first": "wrong", "final": "pass",
               "rounds": [{"usage": {"input": 6000, "output": 100, "steps": 1}, "code": "x"},
                          {"usage": {"input": 7000, "output": 50, "steps": 1}, "code": "xy"}]}
        (line,) = h.summarize([rec], overhead=5000)
        self.assertIn("pass@1 0.000 final 1.000 rounds 2.00 input-net 3000 output 150", line)


if __name__ == "__main__":
    unittest.main()
