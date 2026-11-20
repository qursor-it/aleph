#!/usr/bin/env python3
"""Reference for bench/tasks.md `maxmin`: max and min of integer arguments."""

import sys


def main():
    numbers = [int(arg) for arg in sys.argv[1:]]
    if not numbers:
        sys.stderr.write("no numbers\n")
        sys.exit(1)
    print("max {} min {}".format(max(numbers), min(numbers)))


if __name__ == "__main__":
    main()
