#!/usr/bin/env python3
"""Reference for bench/tasks.md `upper`: uppercase data/in.txt into out/up.txt."""


def ascii_upper(text):
    return "".join(chr(ord(c) - 32) if "a" <= c <= "z" else c for c in text)


def main():
    with open("data/in.txt", encoding="utf-8") as handle:
        text = handle.read()
    with open("out/up.txt", "w", encoding="utf-8", newline="") as handle:
        handle.write(ascii_upper(text))


if __name__ == "__main__":
    main()
