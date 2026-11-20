#!/usr/bin/env python3
"""Reference for bench/tasks.md `points`: polygon perimeter."""
import math


def main():
    points = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0)]
    total = 0.0
    for index, (ax, ay) in enumerate(points):
        bx, by = points[(index + 1) % len(points)]
        dx = ax - bx
        dy = ay - by
        total += math.sqrt(dx * dx + dy * dy)
    print(total)


if __name__ == "__main__":
    main()
