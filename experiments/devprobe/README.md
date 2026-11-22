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
python3 probe.py --tasks tasks2.txt run2.md   # second task set (also run3.md, run5.md, run7.md)
python3 probe.py --tasks tasks3.txt run6.md   # third task set (reference3.md: 20/20)
python3 probe.py --tasks tasks4.txt run8.md   # fourth, harder set (reference4.md: 20/20)
```

`ALEPH` selects the binary (default `target/debug/aleph`).

## Runs

| run | date | model | compiler | clean | hidden pass | errors/problem |
|---|---|---|---|---|---|---|
| run1 | 2026-11-21 | deepseek-v4.1-flash (opencode, researcher, no tools but read) | after ADR-0044 | 16/20 | 16/20 | 0.30 |
| run2 (tasks2) | 2026-11-21 | same | after ADR-0046 | 19/20 | 19/20 | 0.10 |
| run3 (tasks2) | 2026-11-21 | same | T-068 (idioms example in llms.txt) | 18/20 | 17/20 | 0.15 |
| run4 (tasks1) | 2026-11-21 | same | T-069 (`push` note) | 20/20 | 20/20 | 0.00 |
| run5 (tasks2) | 2026-11-21 | same, isolated workspace | T-069 | 18/20 | 18/20 | 0.10 |
| run6 (tasks3) | 2026-11-22 | same, isolated workspace | after ADR-0047 | 19/20 | 19/20 | 0.05 |
| run7 (tasks2) | 2026-11-22 | same, isolated workspace | after ADR-0048 | 20/20 | 20/20 | 0.00 |
| run8 (tasks4) | 2026-11-22 | same, isolated workspace | after ADR-0048 | 17/20 | 17/20 | 0.35 |
| run8 repair | 2026-11-22 | same session, given `aleph check` output (ADR-0049 messages) | T-073 | 20/20 | 20/20 | 0.00 |

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

run3 (same tasks as run2, llms.txt with the `run_idioms` example instead of
`run_eprint`): size 677 vs 702 (-4%), functions 27 vs 31; failures `chunks`
and `cumsum` (recursion `push REC x` builds the list reversed, as `merge` in
run1) and `jname` (passes `jparse`'s own error instead of "bad"). The
change in size is within what one sample per task can show: inconclusive,
kept because the replaced example taught nothing the tasks used.

run4/run5 (llms.txt with `push ... (at end)`): run4 passes all of tasks1
(run1: 16/20, older compiler); run5 fails `chunks` (still `push REC x`,
reversed: the note did not stop it) and `sumints` (`try int` passes
"invalid int" instead of the token). Size run5 594 (run2 702, run3 677),
21 functions (31, 27).

Contamination: the first run5 was written by a session allowed to edit
`experiments/devprobe/`, next to `reference2.md`; 8 of its answers were near
copies of the reference (`chunks` character for character). It was set
aside and rerun from `.agent-tmp/probe-ws/`, a folder holding only
`llms.txt`. The rerun is also close to the reference (mean similarity 0.78,
8 answers >= 0.95; run1-run3 0.52-0.70), so short tasks with one obvious
idiomatic answer converge and similarity alone cannot prove copying. From
run5 on, probes run in the isolated folder.

run6 (tasks3: trees, records, dates, statistics, IPv4, Pascal): only
`inventory` fails, with `match r ok d2 ... err e2 ...` on a `res` (P001:
`match` takes sum types only). Size 823 vs the reference's 646, same number
of functions (22 vs 22). One occurrence of `match` on `res`/`opt` in 120
answers so far: noted, not acted on.

run7 (tasks2 again, after `okor`): all pass; `okor` used 5 times as
intended (`try okor jparse text "bad"`, `mapr lam t okor int t t`). Size on
tasks2 across runs: run2 702, run3 677, run5 594, run7 544 (reference 553);
functions 31, 27, 21, 20.

Limits: one model, one sample per task, tasks chosen by the same agent that
changes the language; a run shows where models stumble, not how often.

run8 (tasks4: harder tasks, written after run7 saturated sets 1-3): three
wrong answers, all caught by the model's own `ex` lines (E002), none a
compile error. `merge` and `transpose` built their lists with `push REC x`
as if `push` prepended, the habit already seen in run1, run3 and run5 that
the "(at end)" note did not remove: ADR-0049 makes E002 say "found is
expected reversed (push appends at the end)". `reachable` never marked the
neighbours as seen (a logic slip). In a repair turn the same session got
the `aleph check` output for the three answers (`run8-fix.md`) and fixed
all three, both list cases with `cat [x] REC`. There is no control turn
with the old message, so this shows the message is usable, not that it is
needed: E002's expected/found alone may have been enough.
