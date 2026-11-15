# Sample benchmark results

- Date: 2026-11-15T20:24Z (UTC)
- Command: `BENCH_RUNS=3 bench/run.sh`
- Machine: shared 2-core arm64 VPS, noisy
- Wall time: bash `EPOCHREALTIME` around the process; median in ms.
- RSS: peak KB from `/usr/bin/time -f "%M"`; `n/a` when it is unavailable.
- Default run excludes the heavy `fib35` row; use `BENCH_SCALE=1` for that.

| task | native ms | interp ms | python ms | c ms | native KB | interp KB | python KB | c KB |
|------|-----------|-----------|-----------|------|-----------|-----------|-----------|------|
| fib | 131.42 | 724.19 | 302.79 | 5.32 | 1536 | 2432 | 7808 | 1152 |
| total | 2.05 | 2.81 | 19.16 | 1.88 | 1280 | 2304 | 8064 | 1152 |
| evens | 2.44 | 2.78 | 19.97 | 1.93 | 1280 | 2304 | 7808 | 1152 |
| primes | 2.16 | 2.93 | 19.66 | 1.91 | 1280 | 2304 | 7808 | 1152 |
| fizzbuzz | 2.05 | 2.50 | 19.58 | 1.95 | 1280 | 2304 | 7808 | 1152 |
| palindrome | 2.09 | 2.56 | 19.68 | 1.84 | 1152 | 2304 | 7808 | 1024 |
| wordcount | 2.19 | 2.66 | 20.01 | 1.91 | 1280 | 2432 | 7808 | 1152 |
| linecount | 1.69 | 2.78 | 19.72 | 1.90 | 1280 | 2432 | 7808 | 1152 |
| maxmin | 2.06 | 2.81 | 19.93 | 1.91 | 1280 | 2432 | 8064 | 1152 |
| points | 2.15 | 3.04 | 19.26 | 1.79 | 1280 | 2560 | 7936 | 1152 |
| reverse | 1.61 | 2.51 | 19.62 | 1.81 | 1152 | 2304 | 7936 | 1024 |
| gcd | 1.76 | 2.54 | 19.18 | 1.90 | 1280 | 2304 | 8064 | 1152 |
| upper | 1.57 | 2.02 | 18.76 | 1.38 | 1280 | 2432 | 7808 | 1024 |
| histogram | 1.94 | 2.81 | 19.77 | 1.82 | 1280 | 2304 | 8064 | 1152 |
| collatz | 2.03 | 2.91 | 19.94 | 1.93 | 1280 | 2432 | 7808 | 1152 |

Only `fib` does enough work to rise clearly above process-start noise; every
other default task is startup-dominated. A separate `BENCH_SCALE=1` sample of
`fib35` gave `native 1430.35 ms`, `interp 8089.43 ms`, `python 3069.04 ms`,
`c 31.81 ms` (`BENCH_RUNS=1`, same machine). See [`README.md`](README.md) for
caveats.
