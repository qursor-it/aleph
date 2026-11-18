"""Offline tests for pilot.py: python3 -m unittest experiments/pilot/tools/test_pilot.py"""
import json
import pathlib
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import pilot  # noqa: E402


class PilotTest(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp())
        shutil.copytree(pilot.TEMPLATE, self.tmp / "_template")
        self.saved = pilot.SESSIONS, pilot.TEMPLATE
        pilot.SESSIONS, pilot.TEMPLATE = self.tmp, self.tmp / "_template"

    def tearDown(self):
        pilot.SESSIONS, pilot.TEMPLATE = self.saved
        shutil.rmtree(self.tmp)

    def fill(self, folder, **changes):
        meta = json.loads((folder / "meta.json").read_text())
        meta.update({"model": "m", "tool": "t", "task": "gcd", "compiler_commit": "abc123",
                     "attempts": 2, "passed": True, "reasoning_visible": True, "translated": False})
        meta.update(changes)
        (folder / "meta.json").write_text(json.dumps(meta))
        (folder / "prompt.md").write_text("write gcd")
        (folder / "trace.md").write_text("```aleph\nfn main -> unit\n print \"x\"\n```\n")
        (folder / "final.aph").write_text("fn main -> unit\n print \"x\"\n")

    def test_new_session_starts_incomplete_then_passes_when_filled(self):
        pilot.new("gcd", "2026-11-18")
        folder = self.tmp / "2026-11-18-gcd"
        self.assertTrue(pilot.problems(folder))
        self.fill(folder)
        self.assertEqual(pilot.problems(folder), [])

    def test_bad_fields_are_reported(self):
        pilot.new("gcd", "2026-11-18")
        folder = self.tmp / "2026-11-18-gcd"
        self.fill(folder, condition="java", attempts="two", translated="maybe")
        text = " ".join(pilot.problems(folder))
        self.assertIn("condition", text)
        self.assertIn("attempts", text)
        self.assertIn("translated", text)

    def test_python_condition_needs_final_py(self):
        pilot.new("gcd", "2026-11-18")
        folder = self.tmp / "2026-11-18-gcd"
        self.fill(folder, condition="python")
        self.assertIn("final.py is missing", pilot.problems(folder))

    def test_translation_hint_counts_code_before_first_aleph_block(self):
        trace = "Let me think.\n```python\ndef gcd(a, b):\n    return a\n```\nNow in Aleph:\n```aleph\nfn gcd a:i64 b:i64 -> i64\n```\n"
        self.assertGreater(pilot.translation_hint(trace), 0)
        self.assertEqual(pilot.translation_hint("```aleph\nfn main -> unit\n```\ndef later():\n"), 0)

    def test_new_rejects_existing_and_bad_slug(self):
        pilot.new("gcd", "2026-11-18")
        with self.assertRaises(SystemExit):
            pilot.new("gcd", "2026-11-18")
        with self.assertRaises(SystemExit):
            pilot.new("Bad Slug", "2026-11-18")


if __name__ == "__main__":
    unittest.main()
