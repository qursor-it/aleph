#!/usr/bin/env python3
"""Reference for bench/tasks.md `wordcount`: count words in data/in.txt."""


def linewords(line):
    return len([word for word in line.strip().split(" ") if word])


def main():
    with open("data/in.txt", encoding="utf-8") as handle:
        text = handle.read()
    print(sum(linewords(line) for line in text.split("\n")))


if __name__ == "__main__":
    main()
