# Freeze checklist: `v0.2-experiment`

The language version used by the experiments (EXP-002: "a frozen tag, `v0.2-experiment`"). A person creates the tag on the commit chosen below; agents only keep this list up to date (T-043).

## Decisions for a person before tagging

- [ ] **T-030 (scoped regions) and T-031 (`net.host`/`fetch`, `time`, `env`)** are listed for the freeze in T-043 but are paused until the rule-reduction pilot is evaluated. Tag without them, or wait? Without T-031 the capabilities `net.host`, `env` and `time` parse but have no builtins.
- [ ] **ADR status.** The v0.2 ADRs below are all `Proposed`. Agents never accept ADRs: decide which ones the tag freezes as they stand.
- [ ] **`llms.txt` budget.** It is at its limit of 2110 tokens (ADR-0026). The experiments use it as the language description, so freeze its text together with the tag.

## In (implemented on `agent/aleph-m3`)

| Feature | ADR | Task |
|---|---|---|
| `dict K V` and its builtins | 0032 | T-027 |
| JSON: `json` type, `jparse`, accessors, `show` | 0033 | T-028 |
| Sum types and exhaustive `match` | 0034 | T-029 |
| Builtins `ord`, `chr` | 0035 | T-044 |
| Builtins `abs min max starts ends uniq map2` | 0036 | (PR #3) |
| Inline lambdas `lam` (interpreter, native, tools) | 0037 | T-045..T-049 |
| Generated C names no longer clash with Tlön | — | T-034 |
| Tooling: `aleph edit`, `aleph mcp`, `aleph tokens`, `aleph llms`, fix objects in diagnostics | 0028–0031 | T-023..T-026 |

## Out (not in the tag)

- T-030 scoped regions and T-031 network/time/env, unless the decision above says otherwise.
- User generics, modules/imports, general loops, LLVM/WASM backends, content addressing (SPEC §13).
- Experiment tools T-035..T-042: their protocols are still `draft`.

## Commands that must pass on the tagged commit

Run from the repository root, with a C compiler (`cc`) installed:

```sh
cd compiler
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
ALEPH_WERROR=1 cargo test --all    # includes interpreter vs native differential tests; generated C warning-free
cd ..
cc -std=c99 -O2 -Wall -Wextra -Werror -pthread -o /tmp/tt tlon/test_tlon.c tlon/tlon.c -lm && /tmp/tt
./target/debug/aleph llms | diff - llms.txt                      # llms.txt is current
cd experiments/benchmarks/humaneval
ALEPH=../../../target/debug/aleph python3 evaluate.py humaneval-aleph.jsonl --validate   # 156 problems, 0 errors
python3 test_humaneval_to_aleph.py
cd ../../..
ALEPH=$PWD/target/debug/aleph python3 experiments/pilot/tokens/verify_whatif.py          # C 61/61 pass
```

State on 19 Nov 2026 (commit after T-050): all of the above pass. `cargo test` runs 356 tests; `verify_whatif.py` gives A 60/61, B 61/61 and C 61/61. A/29 fails because the `starts` builtin is now a reserved name.

## After tagging

- Record the tag and commit hash in each protocol's run notes (people only; agents do not write in `experiments/results/`).
- Language changes after the tag go to a new tag. The experiments keep the frozen one.
