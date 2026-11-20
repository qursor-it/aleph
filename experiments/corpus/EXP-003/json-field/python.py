import json
import pathlib
import sys

data = json.loads(pathlib.Path("input.json").read_text(encoding="utf-8"))
value = data.get(sys.argv[1])
if not isinstance(value, str):
    print("missing", file=sys.stderr)
    sys.exit(1)
print(value)
