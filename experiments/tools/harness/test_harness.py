#!/usr/bin/env python3
"""Offline tests for the EXP-002 harness. A fake model only; no network."""
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

import harness

ROOT = pathlib.Path(__file__).resolve().parents[3]
SAMPLES = ROOT / "experiments" / "tasks" / "EXP-002-samples"
HARNESS = pathlib.Path(__file__).resolve().parent / "harness.py"

PRICES = {"default": {"input": 1.0, "cached_input": 0.5, "output": 2.0}}
USAGE = {"input": 1000, "cached_input": 100, "output": 50, "reasoning": 10}


def block(code, language=""):
    return f"```{language}\n{code.rstrip()}\n```"


def write_json(path, value):
    path.write_text(json.dumps(value), encoding="utf-8")
    return path


def events(path, kind):
    return [record for record in harness.read_records(path) if record.get("event") == kind]


class HarnessTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.dir = pathlib.Path(self.temp.name)
        self.prices = write_json(self.dir / "prices.json", PRICES)
        self.log = self.dir / "log.jsonl"

    def tearDown(self):
        self.temp.cleanup()

    def answers(self, name, entries):
        return write_json(self.dir / name, entries)

    def test_python_passes_in_round_one(self):
        task = SAMPLES / "algorithm-primes"
        solution = (task / "ref" / "solution.py").read_text(encoding="utf-8")
        answers = self.answers("answers.json", [
            {"text": block(solution, "python"), "usage": USAGE},
        ])
        code = harness.main([
            "run", "--task", str(task), "--condition", "python",
            "--model", f"fake:{answers}", "--cap-eur", "1.0",
            "--prices", str(self.prices), "--out", str(self.log),
            "--rounds", "3", "--seed", "7",
        ])
        self.assertEqual(code, 0)

        rounds = events(self.log, "round")
        self.assertEqual(len(rounds), 1)
        record = rounds[0]
        self.assertTrue(record["passed"])
        self.assertTrue(record["check_ok"])
        self.assertEqual(record["tests_passed"], 4)
        self.assertEqual(record["tests_total"], 4)
        self.assertEqual(record["seed"], 7)
        self.assertEqual(record["usage"],
                         {"new_input": 900, "cached_input": 100, "output": 50, "reasoning": 10})

        summary = events(self.log, "summary")[0]
        self.assertTrue(summary["passed"])
        self.assertEqual(summary["rounds_to_pass"], 1)
        self.assertEqual(summary["total_tokens"]["new_input"], 900)
        self.assertAlmostEqual(summary["total_cost_eur"], 0.00107, places=9)

    def test_fails_then_passes_with_feedback(self):
        task = SAMPLES / "algorithm-primes"
        wrong = "import sys\nprint('wrong')\n"
        correct = (task / "ref" / "solution.py").read_text(encoding="utf-8")
        answers = self.answers("answers.json", [
            {"text": block(wrong, "python"), "usage": USAGE},
            {"text": block(correct, "python"), "usage": USAGE},
        ])
        backend = harness.FakeBackend(str(answers))
        harness.run_task(
            task, "python", backend, f"fake:{answers}", 1.0, PRICES, self.log,
            rounds=3, seed=11,
        )

        self.assertEqual(backend.index, 2)
        rounds = events(self.log, "round")
        self.assertFalse(rounds[0]["passed"])
        self.assertLess(rounds[0]["tests_passed"], rounds[0]["tests_total"])
        self.assertTrue(rounds[1]["passed"])

        feedback = backend.calls[1]["messages"][-1]["content"]
        self.assertIn("primes-below-ten", feedback)
        self.assertIn("expected stdout", feedback)
        self.assertEqual(events(self.log, "summary")[0]["rounds_to_pass"], 2)

    def test_missing_cap_exits_two(self):
        result = subprocess.run(
            [sys.executable, str(HARNESS), "run",
             "--task", str(SAMPLES / "algorithm-primes"),
             "--condition", "python", "--model", "fake:x.json",
             "--prices", str(self.prices), "--out", str(self.log)],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("--cap-eur", result.stderr)

    def test_cap_counts_earlier_records_and_stops_before_the_call(self):
        task = SAMPLES / "algorithm-primes"
        solution = (task / "ref" / "solution.py").read_text(encoding="utf-8")
        first = self.answers("first.json", [{"text": block(solution), "usage": USAGE}])
        harness.run_task(task, "python", harness.FakeBackend(str(first)),
                         f"fake:{first}", 1.0, PRICES, self.log, rounds=1)
        spent = harness.previous_spend(self.log)
        self.assertGreater(spent, 0)

        second_answers = self.answers("second.json", [{"text": block(solution), "usage": USAGE}])
        second = harness.FakeBackend(str(second_answers))
        records = harness.run_task(task, "python", second, f"fake:{second_answers}",
                                   spent + 1e-9, PRICES, self.log, rounds=1)

        self.assertEqual(second.calls, [])
        self.assertFalse([record for record in records if record["event"] == "round"])
        stopped = [record for record in records if record["event"] == "stopped"]
        self.assertEqual(len(stopped), 1)
        self.assertEqual(stopped[0]["reason"], "cap")
        self.assertEqual(stopped[0]["message"], "stopped: cap")
        self.assertAlmostEqual(stopped[0]["spend_eur"], spent, places=9)
        self.assertFalse(events(self.log, "summary")[-1]["passed"])

    def test_aleph_prompt_has_llms_and_examples_exclude_current_task(self):
        task = SAMPLES / "algorithm-primes"
        system, user = harness.build_prompt(
            "aleph-llms-examples", task,
            llms_path=ROOT / "llms.txt", examples_dir=SAMPLES)
        self.assertIn("Aleph is a programming language", system)
        self.assertIn("## 5. Expressions", system)

        examples = harness.load_examples(SAMPLES, task)
        ids = [example["id"] for example in examples]
        self.assertLessEqual(len(examples), 5)
        self.assertNotIn("algorithm-primes", ids)
        self.assertIn("### Example", user)
        self.assertIn("data-csv-column-sum", user)
        self.assertNotIn("### Example algorithm-primes", user)
        current_solution = (task / "ref" / "solution.aph").read_text(encoding="utf-8").strip()
        self.assertNotIn(current_solution, user)

        aleph_system, _ = harness.build_prompt("aleph-llms", task, llms_path=ROOT / "llms.txt")
        self.assertIn("## 9. Diagnostics", aleph_system)

    def test_summary_numbers_by_condition(self):
        prime = (SAMPLES / "algorithm-primes" / "ref" / "solution.py").read_text(encoding="utf-8")
        top = (SAMPLES / "text-top-words" / "ref" / "solution.py").read_text(encoding="utf-8")
        wrong = "print('wrong')\n"
        entry = lambda text: {"text": block(text), "usage": USAGE}

        runs = [
            ("algorithm-primes", [entry(prime)]),
            ("text-top-words", [entry(wrong), entry(top)]),
            ("json-field", [entry(wrong), entry(wrong)]),
        ]
        for index, (name, answers_list) in enumerate(runs):
            answers = self.answers(f"answers-{index}.json", answers_list)
            harness.run_task(SAMPLES / name, "python", harness.FakeBackend(str(answers)),
                             f"fake:{answers}", 1.0, PRICES, self.log, rounds=2)

        summary = harness.summarize(self.log)
        stats = summary["by_condition"]["python"]
        self.assertEqual(summary["runs"], 3)
        self.assertEqual(stats["runs"], 3)
        self.assertEqual(stats["solved"], 2)
        self.assertAlmostEqual(stats["pass_at_1"], 1 / 3)
        self.assertAlmostEqual(stats["pass_at_k"], 2 / 3)
        self.assertEqual(stats["k"], 2)
        self.assertAlmostEqual(stats["mean_rounds_to_pass"], 1.5)
        self.assertAlmostEqual(stats["tokens_per_solved"]["new_input"], 1350)
        self.assertAlmostEqual(stats["tokens_per_solved"]["cached_input"], 150)
        self.assertAlmostEqual(stats["tokens_per_solved"]["output"], 75)
        self.assertAlmostEqual(stats["tokens_per_solved"]["reasoning"], 15)
        self.assertAlmostEqual(stats["cost_eur_per_solved"], 0.001605, places=9)
        self.assertAlmostEqual(stats["total_cost_eur"], 0.00535, places=9)

        result = subprocess.run([sys.executable, str(HARNESS), "summary", str(self.log)],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(json.loads(result.stdout)["runs"], 3)

    def test_aleph_condition_with_binary(self):
        aleph = harness.resolve_aleph()
        if not (pathlib.Path(aleph).is_file() or shutil.which(aleph)):
            self.skipTest(f"Aleph binary not found: {aleph}")
        task = SAMPLES / "algorithm-primes"
        solution = (task / "ref" / "solution.aph").read_text(encoding="utf-8")
        answers = self.answers("aleph-answers.json", [{"text": block(solution, "aleph"),
                                                      "usage": USAGE}])
        records = harness.run_task(
            task, "aleph-llms", harness.FakeBackend(str(answers)), f"fake:{answers}",
            1.0, PRICES, self.log, rounds=1, aleph=aleph)
        rounds = [record for record in records if record["event"] == "round"]
        self.assertEqual(len(rounds), 1)
        self.assertTrue(rounds[0]["check_ok"], rounds[0])
        self.assertTrue(rounds[0]["passed"], rounds[0])


if __name__ == "__main__":
    unittest.main()
