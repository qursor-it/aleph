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
python3 probe.py --tasks tasks4.txt run8.md   # fourth, harder set (reference4.md: 20/20; also run9-run11)
python3 pyratio.py py-haiku.md run14-haiku.md run12-haiku.md   # same model, Aleph vs Python size (also py-deepseek.md with run14.md run12.md)
python3 probe.py --tasks tasks5.txt run12.md  # fifth set, written after the T-077 change (reference5.md: 20/20; also run12-haiku.md)
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
| run8 control | 2026-11-22 | fresh session, the 2 list answers with the old E002 text | T-073 | 19/20 | 19/20 | 0.05 |
| run9 (tasks4) | 2026-11-22 | same, isolated workspace | T-074 (llms.txt 8212 chars) | 19/20 | 19/20 | 0.10 |
| run10 (tasks4) | 2026-11-22 | same, isolated workspace | T-077 trial (okor example + "prefer map filter fold mapr" line) | 13/20 | 13/20 | 0.50 |
| run11 (tasks4) | 2026-11-22 | same, isolated workspace | same as run10 | 17/20 | 17/20 | 0.15 |
| run12 (tasks5) | 2026-11-22 | same, isolated workspace | T-077 (okor example only, llms.txt 8203 chars) | 20/20 | 20/20 | 0.00 |
| run12-haiku (tasks5) | 2026-11-22 | claude haiku (Claude Code subagent, read 2 files, no commands) | T-077 | 20/20 | 20/20 | 0.00 |
| run13 (tasks4) | 2026-11-22 | deepseek flash, isolated workspace | T-078 trial (`cat [0] [1 2]` example); scored after ADR-0051 | 15/20 | 14/20 | 0.25 |
| run13-haiku (tasks4) | 2026-11-22 | claude haiku | T-077 llms.txt; scored after ADR-0051 (16/20 before) | 17/20 | 17/20 | 0.40 |
| run14 (tasks4) | 2026-11-22 | deepseek flash, isolated workspace | T-080 (ADR-0051, ADR-0052; llms.txt 8280) | 19/20 | 19/20 | 0.10 |
| run14-haiku (tasks4) | 2026-11-22 | claude haiku | T-080 | 20/20 | 20/20 | 0.00 |
| run15-haiku4 (tasks4) | 2026-11-22 | claude haiku | T-082 (ADR-0053 `nth`; llms.txt 8315) | 20/20 | 20/20 | 0.00 |
| run15-haiku5 (tasks5) | 2026-11-22 | claude haiku | T-082 | 20/20 | 20/20 | 0.00 |
| rep16a/b/c (tasks5) | 2026-11-22 | deepseek flash, isolated workspace, tasks read from TASK.md | T-082 | 18, 20, 19 /20 | 18, 20, 19 /20 | 0.20, 0.00, 0.10 |

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
all three, both list cases with `cat [x] REC`. Control: a fresh session
given the same two list answers with the old message (`example failed`,
same expected/found; `run8-control.md`) fixed both, byte-identical to the
repair turn. So for this model the reversal is obvious from expected/found
and the ADR-0049 message is not needed; it stays as a harmless, free hint
until a person decides (it is Proposed). The probe's real lesson: every
run8 error is caught by the model's own examples, so with one `aleph check`
turn the set is 20/20.

run9 (tasks4, after the T-074 trim of llms.txt prose, 8433 -> 8212 chars):
19/20, answers 8082 chars over 39 functions (run8: 9500 over 43; reference
5396 over 31). One error: `wrap` used `len` on a `str` (T001); ADR-0050
makes that T001 say "str length is `count`". With run-to-run noise of about
2 tasks on set 2 (runs 2, 3, 5, 7: 19, 17, 18, 20), this shows the trim did
no visible harm, not that it helped.

run10 and run11 (tasks4) tested the two candidates of `patterns.md` together:
the `run_idioms` example `total` rewritten as
`let ns try mapr lam t okor int t t fields s`, and a §5 line "Lists:
prefer `map filter fold mapr` to recursion." Result: 13/20 and 17/20
(before: run8 17, run9 19); answer size 8212 and 8317 chars (before 9500,
8082); `okor` used 2 and 1 times (before 0, 0); fold/map/mapr/filter 7 and
9 (before 7, 6). The line changed nothing measurable and was removed; the
example stays because it is shorter than the old one and the only place
`okor` appears outside the table. Six of the run10 errors are again the
`push` reversal (merge, runs, wrap) and two are logic slips in toroman /
fromroman; all would be caught by the model's own `ex` lines in one
`aleph check` turn.

run12 (tasks5, a new set written after the change so it was not tuned on):
DeepSeek 20/20 with 0 errors, 6376 chars over 38 functions against the
reference 3271 over 22 (1.95x): correct but helper-heavy. Claude Haiku on
the same prompt and llms.txt (a second model, run as a Claude Code subagent
told to read only TASK.md and llms.txt and run nothing; 3 tool calls):
20/20, 4081 chars over 23 functions (1.25x), `okor` 6 times. Its `avgstr`
copies the `total` example's shape exactly, a hint that idioms reach models
through examples, not through the builtin table.

run13 (tasks4) tried a prepend example (`ex cat [0] [1 2] = [0 1 2]` next to
`push`) against the `push REC x` reversal: the answers use `cat [x]` 0 times
and `runs` is reversed again, so the line was dropped (T-078). Two of its
errors are new: `match` on an `opt` (`rpn`) and on a `res` (`brackets`),
written `match X some v A none B` / `match X ok v A err e B` as in Rust; run6
`inventory` did the same. ADR-0052 (Proposed) lets `match` take those arms.

run13-haiku (tasks4, Claude Haiku, current llms.txt): 16/20 when written;
5 of its 12 errors were `len` on a `str`, the same T001 as DeepSeek run9
`wrap`. ADR-0051 makes `len` accept a str (bytes, like `count`); rescored
with it, run13-haiku is 17/20 and run9 20/20. All probe scores above were
taken with the compiler of their day except where the table says so.

run14 (tasks4, after ADR-0051 `len` on str and ADR-0052 `match` on
opt/res): DeepSeek 19/20 (one logic slip, `titlecase` does not lowercase),
uses `match .. some x ..`/`ok x` 3 times; Claude Haiku 20/20, uses it 0
times. Answer size did not drop (DeepSeek 8733, Haiku 8309 chars against
the reference 5396): the dummy heads `else at X 0 D` stay (17 and 13).
Set 4 first-attempt scores so far: DeepSeek 17, 19, 13, 17, 15, 19; Haiku
16 (17 rescored), 20. Correctness on first attempt is near the ceiling for
both models; answer size, about 1.5x the reference, is what is left, and
none of the llms.txt text changes tried (T-077, T-078) moved it.

## Aleph vs Python, same model (T-081)
Each model also wrote the 40 tasks of sets 4 and 5 in Python (`py-haiku.md`,
`py-deepseek.md`: same statements, the Aleph header and examples given only
as the spec, one `assert` per example, stdlib only). `pyratio.py` runs every
Python block (all 40/40 pass their asserts, for both models) and compares
its size with the same model's verified Aleph answers (run14 + run12, 40/40
and 39/40 hidden-pass). Characters, not tokens (no real tokenizer offline;
`aleph tokens` also estimates chars/4).

| Aleph written by | logic only (no `ex`/`assert`) | whole answer | median per task (logic) |
|---|---|---|---|
| Claude Haiku | 0.983 | 1.105 | 1.085 |
| DeepSeek flash | 1.109 | 1.235 | 1.142 |
| the agent (reference4/5) vs Haiku's Python | 0.801 | 0.772 | 0.886 |

So the size advantage measured on hand-written programs (EXP-003 corpus
0.745, references 0.77-0.80) does not carry over to model-written code:
with these two models Aleph answers are as long as or longer than their
Python. Where Aleph loses (per-task, Haiku): indexing through `opt`
(`else at else at m i empty i64 i 0` vs `m[i]`, 320 `else at X I D` in all
runs), substrings through a char list (`join slice split s "" 0 i ""` vs
`s[:i]`), helper functions each needing a signature with types and an `ex`
line, and Python's comprehensions/`zip`. Where it wins: sum types and
`match` (runops 0.28, inorder 0.43), `mapr`+`okor` parsing (sumcol 0.42,
lookall 0.58). Caveats: characters are a proxy (Aleph's spaces and short
words may tokenize differently from Python punctuation); Python was checked
only on the visible examples; one prompt per model.

run15 (Haiku, sets 4 and 5, after ADR-0053 `nth`): 40/40. Haiku switched
completely: `nth` 30 + 12 times, `else at` 0 (run14-haiku/run12-haiku: 23
and 8 `else at`, 0 `nth`). Logic size vs its Python did not drop: set 4
1.087 -> 1.164, set 5 0.814 -> 0.888. The mechanical saving (each
`else at X I D` -> `nth X I`, about 8 chars) is about 3% of run14-haiku;
run-to-run size noise for one model on one set is 10-15% (run13-haiku 7264
vs run14-haiku 8309 chars on the same llms.txt minus two lines), so an
effect this small cannot be seen with one run per arm. What the runs do
show: the builtin is adopted at once, causes no error, and removes the
dummy defaults from the code a reader has to check. Measuring size effects
of a few percent needs about ten runs per arm; that is the next methodical
step, not another feature.

## Repetitions (T-083)
Three more DeepSeek runs of set 5 on the current llms.txt (`rep16a-c.md`;
same prompt, read from TASK.md instead of pasted) and two more DeepSeek
Python runs of the 40 tasks (`py-deepseek-a.md`, `-b`; all 40/40 pass).
Ratios of logic size, Aleph/Python, on set 5 tasks where the Aleph answer
passes the hidden examples, each Aleph run against each Python run:

| Aleph run | llms.txt | vs py-deepseek | vs -a | vs -b |
|---|---|---|---|---|
| run12 | T-077 (before ADR-0051..0053) | 1.194 | 1.125 | 1.155 |
| rep16a | T-082 | 0.920 | 0.863 | 0.884 |
| rep16b | T-082 | 0.851 | 0.802 | 0.823 |
| rep16c | T-082 | 1.102 | 1.035 | 1.063 |

The model's Python is stable (3488-3704 chars on set 5, +-3%); its Aleph is
not (2969-4166): the noise is in how the model writes Aleph. With the
current llms.txt DeepSeek's Aleph is 0.80-1.10x its Python (mean about
0.93); run12 on the older text is above all three, weak evidence (one run)
that ADR-0051..0053 helped. Errors in the repetitions are ordinary type and
logic slips (bylen, minmax, rle), none from the new builtins; `nth` is used
6-19 times per run and `else at` never.
