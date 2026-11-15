#!/usr/bin/env python3
"""Reference for bench/tasks.md `collatz`: sequence lengths for 1..20."""


def collatz_length(n):
    length = 1
    while n != 1:
        n = n // 2 if n % 2 == 0 else 3 * n + 1
        length += 1
    return length


def main():
    for n in range(1, 21):
        print(collatz_length(n))


if __name__ == "__main__":
    main()
