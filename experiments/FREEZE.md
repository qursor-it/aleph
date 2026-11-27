# Freeze checklist: `v0.2-experiment`

The language version used by the experiments (EXP-002: "a frozen tag, `v0.2-experiment`"). A person creates the tag on the commit chosen below; agents only keep this list up to date (T-043).

## Decisions for a person before tagging

- [x] **ADR status.** Decided by the owner on 27 Nov 2026: ADRs 0032-0053 below are `Accepted` and frozen as they stand. ADR-0054 (helpers without examples, branch `agent/helper-lite`) stays `Proposed` and out of the tag.
- [x] **`llms.txt` budget.** Decided by the owner on 27 Nov 2026: the limit stays 2110 tokens (ADR-0026) and the text is frozen with the tag. Its limit is 2110 tokens; 8315/8440 chars after T-074, T-077, T-079, T-080 and T-082 (about 31 tokens free). The experiments use it as the language description, so freeze its text together with the tag. Every feature since JSON paid for its lines by trimming prose; raising the budget is a decision for a person.

## In (implemented on `agent/aleph-m3`)

| Feature | ADR | Task |
|---|---|---|
| `dict K V` and its builtins | 0032 | T-027 |
| JSON: `json` type, `jparse`, accessors, `show` | 0033 | T-028 |
| Sum types and exhaustive `match` | 0034 | T-029 |
| Builtins `ord`, `chr` | 0035 | T-044 |
| Builtins `abs min max starts ends uniq map2` | 0036 | (PR #3) |
| Inline lambdas `lam` (interpreter, native, tools) | 0037 | T-045..T-049 |
| Builtins `env`, `now`, `fetch` (curl) for `use env`, `use time`, `use net.host` | 0038 | T-031 |
| Scoped regions per call, lambda regions, compaction (runtime only) | 0039 | T-030, T-053, T-058 |
| `aleph check` reports layout as an F001 warning (`--strict` rejects) | 0040 | T-055 |
| `[]` empty list typed from context | 0041 | T-056 |
| Builtins `mapr sortby repeat index` | 0042 | T-057, T-060, T-061 |
| Builtin `sqrt` | 0043 | T-064 |
| Guard lines `if C E` (early return in a body) | 0044 | T-065 |
| `cat` on lists | 0045 | T-066 |
| Builtin `tof` (i64 -> f64) | 0046 | T-067 |
| Builtin `fields` (split on whitespace) | 0047 | T-071 |
| Builtin `okor` (opt/res with a chosen error) | 0048 | T-072 |
| E002 names a list found reversed | 0049 | T-073 |
| T001 names `count`/`len` for the other length | 0050 | T-076 |
| `len` also takes a `str` (bytes, like `count`) | 0051 | T-079 |
| `match` on `opt`/`res` (`some x A none B`, `ok x A err e B`) | 0052 | T-080 |
| Builtin `nth` (list index, R006 out of range) | 0053 | T-082 |
| llms.txt: idiomatic loop example, `push` note | — | T-068, T-069 |
| Static tables for literal lists in generated C | — | T-062 |
| Generated C names no longer clash with Tlön | — | T-034 |
| Tooling: `aleph edit`, `aleph mcp`, `aleph tokens`, `aleph llms`, fix objects in diagnostics | 0028–0031 | T-023..T-026 |

## Out (not in the tag)

- User generics, modules/imports, general loops, LLVM/WASM backends, content addressing (SPEC §13).

## Commands that must pass on the tagged commit

Run from the repository root, with a C compiler (`cc`) installed:

```sh
cd compiler
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
ALEPH_REQUIRE_CC=1 cargo test --all   # interpreter vs native; generated C must be -Werror clean (ALEPH_WERROR=0 to opt out on other compilers); fetch tests need curl
cd ..
cc -std=c99 -O2 -Wall -Wextra -Werror -pthread -o /tmp/tt tlon/test_tlon.c tlon/tlon.c -lm && /tmp/tt
./target/debug/aleph llms | diff - llms.txt                      # llms.txt is current
cd experiments/benchmarks/humaneval
ALEPH=../../../target/debug/aleph python3 evaluate.py humaneval-aleph.jsonl --validate   # 156 problems, 0 errors
python3 test_humaneval_to_aleph.py
cd ../../..
ALEPH=$PWD/target/debug/aleph python3 experiments/pilot/tokens/verify_whatif.py          # C 61/61 pass
```

State on 27 Nov 2026 (ADRs 0032-0053 accepted, T-085): all of the above pass. `cargo test` runs 415 tests (one machine-wide `cargo test` at a time, ~/.cargo/bin/cargo lock); `verify_whatif.py` gives A 60/61, B 61/61 and C 61/61 (A/29 uses the reserved name `starts`). Experiment protocols EXP-001..004 are `approved` (owner delegation of 20 Nov) and their tools are built (T-035..T-042).

## After tagging

- Record the tag and commit hash in each protocol's run notes (people only; agents do not write in `experiments/results/`).
- Language changes after the tag go to a new tag. The experiments keep the frozen one.
