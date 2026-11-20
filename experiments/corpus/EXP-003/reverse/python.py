#!/usr/bin/env python3
"""Reference for bench/tasks.md `reverse`: reverse the words of each argument."""

import sys


def main():
    for arg in sys.argv[1:]:
        print(" ".join(reversed(arg.strip().split(" "))))


if __name__ == "__main__":
    main()
