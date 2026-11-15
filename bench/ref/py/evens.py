#!/usr/bin/env python3
"""Reference for bench/tasks.md `evens`: even numbers from 0 to 20."""


def main():
    print(" ".join(str(n) for n in range(21) if n % 2 == 0))


if __name__ == "__main__":
    main()
