#!/usr/bin/env python3
"""Reference for bench/tasks.md `primes`: all primes below 100."""


def is_prime(n):
    if n < 2:
        return False
    d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d += 1
    return True


def main():
    for n in range(2, 100):
        if is_prime(n):
            print(n)


if __name__ == "__main__":
    main()
