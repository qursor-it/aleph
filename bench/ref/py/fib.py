#!/usr/bin/env python3
"""Reference for bench/tasks.md `fib`: print the 30th Fibonacci number."""


def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def main():
    print(fib(30))


if __name__ == "__main__":
    main()
