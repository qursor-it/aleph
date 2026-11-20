"""Offline tests for traces.py: python3 -m unittest -v test_traces."""
import json
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import traces  # noqa: E402

HERE = pathlib.Path(__file__).resolve().parent
FIXTURES = HERE / "fixtures"


def span(role, kind, text, tool=None):
    return {"role": role, "kind": kind, "tool": tool, "text": text, "cached": False}


class ImportTest(unittest.TestCase):
    def test_import_opencode(self):
        sessions = traces.import_opencode(str(FIXTURES / "opencode_events.jsonl"), "script")
        self.assertEqual(len(sessions), 1)
        session = sessions[0]
        self.assertEqual(session["session"], "ses_1")
        self.assertEqual(session["source"], "opencode")
        self.assertEqual(session["task_type"], "script")
        self.assertTrue(session["reasoning_visible"])
        kinds = {item["kind"] for item in session["spans"]}
        self.assertEqual(kinds, {"text", "reasoning", "tool_call", "tool_result"})
        calls = [item for item in session["spans"] if item["kind"] == "tool_call"]
        self.assertEqual([item["tool"] for item in calls], ["bash", "read"])
        self.assertEqual(json.loads(calls[0]["text"])["command"], "python3 x.py")
        results = [item for item in session["spans"] if item["kind"] == "tool_result"]
        self.assertEqual([(item["tool"], item["text"]) for item in results],
                         [("bash", "hi\n"), ("read", "print('hi')\n")])

    def test_import_claude_code(self):
        sessions = traces.import_claude_code(str(FIXTURES / "claude_code_session.jsonl"))
        self.assertEqual(len(sessions), 1)
        session = sessions[0]
        self.assertEqual(session["session"], "cc_1")
        self.assertEqual(session["source"], "claude-code")
        self.assertTrue(session["reasoning_visible"])
        self.assertTrue(any(item["role"] == "system" and item["text"] == "You are an agent."
                            for item in session["spans"]))
        calls = [item for item in session["spans"] if item["kind"] == "tool_call"]
        self.assertEqual([item["tool"] for item in calls], ["Write", "Read", "Bash"])
        results = [item for item in session["spans"] if item["kind"] == "tool_result"]
        self.assertEqual([item["tool"] for item in results], ["Write", "Read", "Bash"])
        self.assertEqual(results[1]["text"], "print('hi')\n")

    def test_import_jsonl_schema(self):
        session = {"session": "s", "source": "other", "task_type": None, "reasoning_visible": True,
                   "spans": [span("assistant", "text", "hello")]}
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory, "roundtrip.jsonl")
            path.write_text(json.dumps(session) + "\n", encoding="utf-8")
            loaded = traces.import_jsonl(str(path), "data")
        self.assertEqual(loaded[0]["session"], "s")
        self.assertEqual(loaded[0]["task_type"], "data")
        self.assertEqual(loaded[0]["spans"], session["spans"])


class ClassifierTest(unittest.TestCase):
    def test_every_class(self):
        items = [
            span("system", "text", "spec"),
            span("user", "text", "do it"),
            span("assistant", "reasoning", "hmm"),
            span("assistant", "text", "before\n```python\nprint(1)\n```\nafter"),
            span("assistant", "tool_call", json.dumps({"content": "x = 1\n", "file_path": "a.py"}), "write"),
            span("assistant", "tool_call", json.dumps({"file_path": "a.py"}), "read"),
            span("tool", "tool_result", "x = 1\n", "read"),
            span("tool", "tool_result", "ok", "bash"),
        ]
        found = {name for item in items for name, _ in traces.classify(item)}
        self.assertEqual(found, set(traces.CLASSES))

    def test_text_split_and_tool_rules(self):
        pieces = traces.classify(span("assistant", "text", "before\n```python\nprint(1)\n```\nafter"))
        self.assertIn(("prose", "before\n"), pieces)
        self.assertIn(("code_written", "print(1)\n"), pieces)
        self.assertIn(("prose", "\nafter"), pieces)
        write = traces.classify(span("assistant", "tool_call",
                                     json.dumps({"content": "x = 1\n", "file_path": "a.py"}), "write"))
        self.assertIn(("code_written", "x = 1\n"), write)
        self.assertTrue(any(name == "tool_other" and "file_path" in text for name, text in write))
        read_args = traces.classify(span("assistant", "tool_call",
                                         json.dumps({"file_path": "a.py"}), "read"))
        self.assertTrue(all(name == "tool_other" for name, _ in read_args))
        self.assertEqual(traces.classify(span("tool", "tool_result", "x = 1\n", "read")),
                         [("code_read", "x = 1\n")])
        self.assertEqual(traces.classify(span("tool", "tool_result", "ok", "bash")),
                         [("tool_other", "ok")])


class CounterTest(unittest.TestCase):
    def test_bpe_on_tiny_vocab(self):
        counter = traces.TokenCounter.load(str(FIXTURES / "tiny.tiktoken"))
        self.assertEqual(counter.count("he"), 1)
        self.assertEqual(counter.count("hell"), 1)
        self.assertEqual(counter.count("hello"), 1)
        self.assertEqual(counter.count("hellohello"), 2)
        self.assertTrue(counter.label.startswith("bpe"))

    def test_chars4_estimate_labelled(self):
        counter = traces.TokenCounter.estimate()
        self.assertEqual(counter.label, "estimate (chars/4)")
        self.assertEqual(counter.count(""), 0)
        self.assertEqual(counter.count("abc"), 1)
        self.assertEqual(counter.count("abcd"), 1)
        self.assertEqual(counter.count("abcde"), 2)
        self.assertEqual(counter.count("abcdefgh"), 2)


class ReportTest(unittest.TestCase):
    def sessions(self):
        return [
            {"session": "a", "source": "opencode", "task_type": "script", "reasoning_visible": True,
             "spans": [span("assistant", "reasoning", "aaaa"),
                       span("assistant", "text", "bbbb"),
                       span("assistant", "text", "```python\nx = 1\n```"),
                       span("tool", "tool_result", "y = 2\n", "read")]},
            {"session": "b", "source": "opencode", "task_type": "repo", "reasoning_visible": True,
             "spans": [span("assistant", "reasoning", "cccccccc"),
                       span("assistant", "text", "dddd")]},
            {"session": "c", "source": "claude-code", "task_type": "repo", "reasoning_visible": False,
             "spans": [span("assistant", "text", "eeeeeeeeeeee")]},
        ]

    def test_bootstrap_is_deterministic(self):
        first = traces.report(self.sessions(), seed=7, boot=100)
        second = traces.report(self.sessions(), seed=7, boot=100)
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        self.assertEqual(first["seed"], 7)
        self.assertEqual(first["boot"], 100)

    def test_no_reasoning_sessions_are_separate(self):
        result = traces.report(self.sessions(), seed=0, boot=50)
        self.assertEqual(result["sessions"], {"visible": 2, "no_reasoning": 1})
        self.assertEqual(result["no_reasoning"]["total"], 3)
        self.assertEqual(result["no_reasoning"]["tokens"]["prose"], 3)
        self.assertNotIn("eeeeeeeeeeee", json.dumps(result["overall"]))
        self.assertEqual(result["overall"]["total"], sum(result["overall"]["tokens"].values()))

    def test_s_max(self):
        result = traces.report(self.sessions(), seed=0, boot=20)
        code = (result["overall"]["pooled"]["shares"]["code_written"]
                + result["overall"]["pooled"]["shares"]["code_read"])
        self.assertGreater(code, 0)
        for r in (0.5, 0.7, 0.9):
            self.assertAlmostEqual(result["s_max"][str(r)], code * (1 - r))


if __name__ == "__main__":
    unittest.main()
