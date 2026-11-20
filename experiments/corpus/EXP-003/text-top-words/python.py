import collections
import pathlib

words = pathlib.Path("text.txt").read_text(encoding="utf-8").split()
counts = collections.Counter(words)
ranked = sorted(counts.items(), key=lambda item: (-item[1], item[0]))
for word, count in ranked[:3]:
    print(f"{word} {count}")
