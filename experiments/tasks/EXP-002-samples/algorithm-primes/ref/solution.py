import sys

n = int(sys.argv[1])
primes = []
for candidate in range(2, n):
    divisor = 2
    while divisor * divisor <= candidate and candidate % divisor != 0:
        divisor += 1
    if divisor * divisor > candidate:
        primes.append(str(candidate))
if primes:
    sys.stdout.write("\n".join(primes) + "\n")
