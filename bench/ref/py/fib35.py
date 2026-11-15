#!/usr/bin/env python3
"""Heavy reference for BENCH_SCALE=1: print the 35th Fibonacci number."""


def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def main():
    print(fib(35))


if __name__ == "__main__":
    main()
