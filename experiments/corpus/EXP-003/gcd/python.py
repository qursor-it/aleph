#!/usr/bin/env python3
"""Reference for bench/tasks.md `gcd`: gcd and lcm of two integer arguments."""

import math
import sys


def main():
    a = int(sys.argv[1])
    b = int(sys.argv[2])
    g = math.gcd(a, b)
    l = 0 if a == 0 or b == 0 else a // g * b
    print("gcd {} lcm {}".format(g, l))


if __name__ == "__main__":
    main()
