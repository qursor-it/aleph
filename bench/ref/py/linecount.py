#!/usr/bin/env python3
"""Reference for bench/tasks.md `linecount`: non-empty lines in data/in.txt."""


def main():
    with open("data/in.txt", encoding="utf-8") as handle:
        text = handle.read()
    print(len([line for line in text.split("\n") if line.strip()]))


if __name__ == "__main__":
    main()
