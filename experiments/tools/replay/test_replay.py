import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import replay

GOOD = "fn add a:i64 b:i64 -> i64\n ex add 2 3 = 5\n + a b\n"
SPACED = "fn add a:i64 b:i64 -> i64\n  ex add 2 3 = 5\n  + a b\n"
EMPTY = "fn e -> [i64]\n ex e = []\n []\n"


@unittest.skipUnless(os.path.exists(replay.aleph_bin()), "aleph binary missing")
class ReplayTest(unittest.TestCase):
    def test_report(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "fx.jsonl")
            rows = [{"name": "a", "code": GOOD}, {"name": "b", "code": SPACED},
                    {"name": "c", "code": EMPTY}, {"name": "d", "code": ""},
                    {"name": "p", "language": "python", "code": "x=1"}]
            with open(path, "w") as f:
                f.write("\n".join(json.dumps(r) for r in rows) + "\n")
            (g,) = replay.build_report([path], replay.aleph_bin())
        self.assertEqual((g["answers"], g["no_code"]), (4, 1))
        self.assertEqual(g["clean_raw_pct"], 33.3)
        self.assertEqual(g["clean_fmt_pct"], 66.7)
        self.assertEqual(g["codes_raw"].get("F001"), 1)
        self.assertEqual(g["codes_fmt"].get("P001"), 1)
        self.assertEqual([f["name"] for f in g["failing_fmt"]], ["c"])
        self.assertIn("clean after fmt", replay.render([g]))


if __name__ == "__main__":
    unittest.main()
