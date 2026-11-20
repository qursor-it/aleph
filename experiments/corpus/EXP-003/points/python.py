#!/usr/bin/env python3
"""Reference for bench/tasks.md `points`: polygon perimeter."""


def newton_sqrt(x):
    """Same 40-iteration Newton iteration as the Aleph program's `sqrt`."""
    if x <= 0.0:
        return 0.0
    guess = x
    for _ in range(40):
        guess = (guess + x / guess) / 2.0
    return guess


def main():
    points = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0)]
    total = 0.0
    for index, (ax, ay) in enumerate(points):
        bx, by = points[(index + 1) % len(points)]
        dx = ax - bx
        dy = ay - by
        total += newton_sqrt(dx * dx + dy * dy)
    print(total)


if __name__ == "__main__":
    main()
