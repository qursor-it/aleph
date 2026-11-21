# Dev probe (agent, not an experiment)

A small design aid, not EXP-002: 20 function tasks written by the agent
(`tasks.txt`: statement, header, visible examples, hidden examples), answered
by one model session that read only `llms.txt`, then compiled and run against
the hidden examples. It finds gaps in the language and in llms.txt; it is not
a measure of Aleph against Python and its numbers are not reported as one.

Why it exists: after ADR-0040..0044 the recorded pilot answers have almost no
language-caused errors left, so new evidence was needed to pick the next
change. The project owner allowed DeepSeek use through opencode (21 Nov
2026, "budget infinito opencode"); the protocol rule that agents run no
experiment is untouched: no task here comes from an experiment suite, no
hidden test of EXP-002 or HumanEval was read, nothing is written in
`experiments/results/`.

```sh
python3 probe.py prompt          # the task text given to the model
python3 probe.py reference.md    # the agent's reference answers: 20/20
python3 probe.py run1.md         # a recorded model run
python3 probe.py --tasks tasks2.txt run2.md   # second task set
```

`ALEPH` selects the binary (default `target/debug/aleph`).

## Runs

| run | date | model | compiler | clean | hidden pass | errors/problem |
|---|---|---|---|---|---|---|
| run1 | 2026-11-21 | deepseek-v4.1-flash (opencode, researcher, no tools but read) | after ADR-0044 | 16/20 | 16/20 | 0.30 |
| run2 (tasks2) | 2026-11-21 | same | after ADR-0046 | 19/20 | 19/20 | 0.10 |

run1 failures: `rotate` (`cat` on lists, T001: ADR-0045), `merge` (built
reversed with `push`, no way to concatenate: ADR-0045), `rle` (wrong own
example), `caesar` (a two-argument `if` as the result, P002). The model also
wrote an `i64 -> f64` helper through text twice and called it `tof`
(ADR-0046).

run2 failure: `sumints` folds with a `res` accumulator and a plain step
function (T001/T010); `mapr` would have done it.

Size: the model's answers are longer than the agent's reference answers
(chars/4 without `ex` lines: run1 847 vs 584, run2 702 vs 553) and have about
twice the functions (44 vs 22, 31 vs 21): loops become recursive helpers
walking an index (`fnh xs 0`, `cih xs p 0`) where `filter lam`/`map lam`
over the list or `range 0 len xs` does it in one line. llms.txt shows `lam`
in one example only.

Limits: one model, one sample per task, tasks chosen by the same agent that
changes the language; a run shows where models stumble, not how often.
