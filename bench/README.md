# Benchmark harness (T-021)

Compares four implementations of the 15 programs in [`tasks.md`](tasks.md):

- **native** — `aleph build` (C backend + Tlön) run directly;
- **interp** — `aleph run` (reference interpreter);
- **python** — `bench/ref/py/NAME.py` (Python 3, standard library only);
- **c** — `bench/ref/c/NAME.c` (C99, libc) compiled with `cc -O2`.

The Aleph sources are the goldens in `tests/aph/bench_NAME.aph`; they are used
directly and never duplicated here.

## Running

```sh
bench/run.sh
BENCH_RUNS=5 bench/run.sh
BENCH_TASKS="fib primes" bench/run.sh
BENCH_SCALE=1 bench/run.sh              # adds the heavy fib35 row
ALEPH=target/release/aleph bench/run.sh # skip the release build
```

`bench/run.sh` builds `target/release/aleph` once with `CARGO_BUILD_JOBS=1` and
`nice` (unless `ALEPH` points at an existing binary). For each task it creates a
temporary directory with `data/` and `out/`, copies the golden `.args`, compiles
the C reference and the Aleph program, then:

1. runs each implementation once and compares stdout, stderr, exit code and any
   `.files` outputs with the golden `.out`, `.err`, `.exit` and
   `tests/aph/expect/`; a mismatch aborts with a non-zero exit;
2. runs each implementation `BENCH_RUNS` times (default 3), measuring wall time
   with bash `EPOCHREALTIME` (microseconds) around the process itself and
   reporting the median in milliseconds with two decimals;
3. runs each implementation once more under `/usr/bin/time -f "%M"` to read its
   peak RSS in KB.

Wall timing never uses `/usr/bin/time`, so the time-utility overhead is not
counted. If `/usr/bin/time` is missing, the RSS columns are `n/a` (timing still
works). `python3` is optional: without it the Python columns are `n/a`.

The markdown table has one row per task:

```
| task | native ms | interp ms | python ms | c ms | native KB | interp KB | python KB | c KB |
```

Times include process startup, dynamic linking and (for the interpreter and
Python) parsing. RSS is the peak resident set size of that engine.

## Scaling

`BENCH_SCALE=1` appends one bench-only heavy row, `fib35`, built from
`bench/heavy/fib35.aph` with references under `bench/ref/{c,py}/fib35.*`. It is
the only workload large enough to stay above process-start noise; all the other
tasks are deliberately tiny and startup-dominated. The default run does not
include `fib35` so it stays short.

## Caveats

- Apart from `fib`/`fib35`, the workloads are tiny: process startup, dynamic
  linking and (for the interpreter and Python) parsing dominate. Treat
  differences of a few milliseconds as noise.
- The machine is a shared 2-core arm64 VPS. Other sessions, `cargo`, `cc` or
  the scheduler can distort any individual run; treat
  [`results-sample.md`](results-sample.md) as an illustrative snapshot only.
- Compilation time is deliberately excluded: only the built programs are timed.
  Aleph and C builds happen once per task before the timed loop.
- The references reproduce the golden outputs for the fixture argument ranges
  (`.args`, `data/in.txt`) only. They do not model Aleph's checked-arithmetic
  R001 overflow traps; the fixture values do not overflow.
