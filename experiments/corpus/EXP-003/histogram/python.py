#!/usr/bin/env python3
"""Reference for bench/tasks.md `histogram`: one bar per integer argument."""

import sys


def main():
    for arg in sys.argv[1:]:
        n = int(arg)
        print("{} {}".format(n, "*" * n))


if __name__ == "__main__":
    main()
