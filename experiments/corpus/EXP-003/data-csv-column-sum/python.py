import pathlib
import sys

rows = pathlib.Path("data.csv").read_text(encoding="utf-8").splitlines()
header = rows[0].split(",")
index = header.index(sys.argv[1])
total = 0
for row in rows[1:]:
    if row:
        total += int(row.split(",")[index])
print(total)
