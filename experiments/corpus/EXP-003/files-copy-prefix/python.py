import pathlib
import sys

prefix = sys.argv[1]
lines = pathlib.Path("in.txt").read_text(encoding="utf-8").split("\n")
if lines and lines[-1] == "":
    lines = lines[:-1]
kept = [line for line in lines if line.startswith(prefix)]
pathlib.Path("out.txt").write_text("".join(line + "\n" for line in kept), encoding="utf-8")
