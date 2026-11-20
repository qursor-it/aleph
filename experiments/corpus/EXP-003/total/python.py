#!/usr/bin/env python3
"""Reference for bench/tasks.md `total`: sum the integer arguments."""

import sys


def main():
    print(sum(int(arg) for arg in sys.argv[1:]))


if __name__ == "__main__":
    main()
