#!/usr/bin/env python3
"""Offline tests for the Orbis generator. Fake model only; no network."""
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

import orbis

ROOT = pathlib.Path(__file__).resolve().parents[3]
SAMPLES = ROOT / "experiments" / "tasks" / "EXP-002-samples"
ORBIS = pathlib.Path(__file__).resolve().parent / "orbis.py"


def _aleph():
    path = os.environ.get("ALEPH")
    if path and pathlib.Path(path).is_file():
        return path
    default = pathlib.Path(orbis.DEFAULT_ALEPH)
    return str(default) if default.is_file() else None


def block(code):
    return f"```aleph\n{code.rstrip()}\n```"


def write_json(path, value):
    path.write_text(json.dumps(value), encoding="utf-8")
    return path


def write_statements(path, statements):
    path.write_text(
        "".join(json.dumps(statement) + "\n" for statement in statements),
        encoding="utf-8")
    return path


def jsonl(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()
            if line.strip()]


class OrbisTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.dir = pathlib.Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def answers(self, entries):
        return write_json(self.dir / "answers.json", entries)

    def statements(self, entries):
        return write_statements(self.dir / "statements.jsonl", entries)

    @unittest.skipUnless(_aleph() is not None, "aleph binary not available")
    def test_broken_then_fixed_makes_a_repair_triple(self):
        broken = 'fn main -> unit\n print + "a" 1\n'
        fixed = 'fn main -> unit\n print "ok"\n'
        statements = self.statements([
            {"id": "repair-me", "statement": "Print ok.", "category": "text"},
        ])
        answers = self.answers([block(broken), block(fixed)])
        out = self.dir / "out"

        summary = orbis.generate(
            statements, f"fake:{answers}", out, attempts=2, aleph=_aleph())

        self.assertEqual(summary["kept"], 1)
        self.assertEqual(summary["repairs"], 1)
        self.assertEqual(summary["failed"], 0)
        self.assertEqual((out / "programs" / "repair-me.aph").read_text(encoding="utf-8"),
                         fixed)
        index = jsonl(out / "index.jsonl")
        self.assertEqual([item["id"] for item in index], ["repair-me"])
        repairs = jsonl(out / "repairs.jsonl")
        self.assertEqual(len(repairs), 1)
        repair = repairs[0]
        self.assertEqual(repair["id"], "repair-me")
        self.assertEqual(repair["broken"], broken.rstrip())
        self.assertEqual(repair["fixed"], fixed)
        self.assertTrue(any(item.get("severity") == "error"
                            for item in repair["diagnostics"]))
        # Diagnostics use a stable relative file name, so runs are reproducible.
        self.assertTrue(all(item.get("file") == "program.aph"
                            for item in repair["diagnostics"]))

    @unittest.skipUnless(_aleph() is not None, "aleph binary not available")
    def test_valid_non_canonical_answer_is_kept_canonical(self):
        non_canonical = 'fn main -> unit\n\n print "hi"\n'
        canonical = 'fn main -> unit\n print "hi"\n'
        statements = self.statements([
            {"id": "canonical", "statement": "Print hi.", "category": "text"},
        ])
        answers = self.answers([block(non_canonical)])
        out = self.dir / "out"

        summary = orbis.generate(
            statements, f"fake:{answers}", out, attempts=1, aleph=_aleph())

        self.assertEqual(summary["kept"], 1)
        self.assertEqual(summary["repairs"], 0)
        saved = (out / "programs" / "canonical.aph").read_text(encoding="utf-8")
        self.assertEqual(saved, canonical)
        self.assertTrue(saved.endswith("\n"))

    def test_statement_similar_to_exp002_sample_is_deduplicated(self):
        sample = orbis.load_json(SAMPLES / "algorithm-primes" / "task.json")
        statements = self.statements([
            {"id": "leak", "statement": sample["statement"],
             "category": "algorithm"},
        ])
        # No scripted answers: a dropped statement must never reach the model.
        answers = self.answers([])
        out = self.dir / "out"

        summary = orbis.generate(
            statements, f"fake:{answers}", out, attempts=1,
            dedup_against=[str(SAMPLES)])

        self.assertEqual(summary["kept"], 0)
        self.assertEqual(summary["dropped"], 1)
        self.assertEqual(summary["dropped_details"][0]["id"], "leak")
        self.assertEqual(summary["dropped_details"][0]["nearest_id"], "algorithm-primes")
        self.assertGreaterEqual(summary["dropped_details"][0]["score"], 0.5)
        self.assertEqual(jsonl(out / "index.jsonl"), [])

    def test_cap_is_required_for_non_fake_backend(self):
        statements = self.statements([
            {"id": "x", "statement": "Print x.", "category": "text"},
        ])
        result = subprocess.run(
            [sys.executable, str(ORBIS), "generate", str(statements),
             "--model", "openai:a-model", "--out-dir", str(self.dir / "out")],
            capture_output=True, text=True,
            env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("--cap-eur", result.stderr)


if __name__ == "__main__":
    unittest.main()
