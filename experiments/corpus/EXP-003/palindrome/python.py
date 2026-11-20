#!/usr/bin/env python3
"""Reference for bench/tasks.md `palindrome`: yes/no per argument, ASCII case."""

import sys


def ascii_lower(text):
    return "".join(chr(ord(c) + 32) if "A" <= c <= "Z" else c for c in text)


def palindrome(text):
    lowered = ascii_lower(text)
    return lowered == lowered[::-1]


def main():
    for arg in sys.argv[1:]:
        print("yes" if palindrome(arg) else "no")


if __name__ == "__main__":
    main()
