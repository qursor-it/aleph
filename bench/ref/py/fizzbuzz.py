#!/usr/bin/env python3
"""Reference for bench/tasks.md `fizzbuzz`: fizzbuzz from 1 to 30."""


def fizz(n):
    if n % 15 == 0:
        return "fizzbuzz"
    if n % 3 == 0:
        return "fizz"
    if n % 5 == 0:
        return "buzz"
    return str(n)


def main():
    for n in range(1, 31):
        print(fizz(n))


if __name__ == "__main__":
    main()
