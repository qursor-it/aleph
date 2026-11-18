---
id: EXP-003
title: The same program in four languages
status: draft
question: How many tokens does the same verified program take in Aleph, Python, bash and C, counted with real tokenizers?
owner: Bernardo Manfriani
updated: 2026-11-18
---

# EXP-003 · The same program in four languages

## Question

For programs that do the same thing and pass the same tests, how many tokens does each language need, with the tokenizers models actually use? This measures the ratio `r` that EXP-001 needs to turn its split into a saving.

## Hypotheses

- **H3.1**: the median Aleph/Python token ratio is at most 0.8, counting only logic (no comments, no examples).
- **H3.2**: counting Aleph's `ex` lines, which the other languages do not have, the ratio is close to 1.
- **H3.3**: the cost of loading `llms.txt` is paid back after a number of programs per session that is small enough for real sessions (the break-even `N`).

## Setup

- **Stage A, now**: the 3 benchmark programs in `bench/` and the reference solutions of the EXP-002 task suite, each written and verified in Aleph, Python, bash (where natural) and C.
- **Stage B, after modules**: small real programs (command-line utilities, data scripts) ported by hand, verified against their original tests.
- **Stage C, later**: parts of well-known repositories of different sizes, ported and verified. This needs modules and a larger standard library, and a porting protocol of its own written before it starts.
- **Normalisation**: canonical formatting for Aleph; one fixed formatter per other language (for example `ruff format`, `shfmt`, `clang-format`); comments and docstrings removed; programs counted with and without Aleph examples.
- **Tokenizers**: at least `o200k_base` and the public tokenizers of the open-weights models used in EXP-002 and EXP-004.

## Baselines

Python is the reference; bash and C are reported for context.

## Metrics

- Tokens per program and language, per tokenizer.
- Ratio to Python, median and distribution, with and without examples.
- Break-even `N = tokens(llms.txt) / (tokens(Python) − tokens(Aleph))` per program class.

## Analysis

Paired by program. Medians with bootstrap intervals; Wilcoxon signed-rank test on the per-program difference; results by program category and size.

## Success criterion and decision

- H3.1 holds if the 95% interval of the median ratio is entirely below 0.8.
- If the ratio is above 0.9, compact syntax is not an argument for Aleph: the keywords and builtins with the worst token cost are revised (the spec says their spelling is provisional) and the experiment is repeated once.

## Cost and limits

No model calls. Real tokenizers need their vocabulary files, downloaded once by a person and given by path (ADR-0031).

## Tools to build

- **T-039**: BPE counter in `aleph tokens` reading a local vocabulary file given by path (follow-up to ADR-0031).
- **T-040**: corpus layout (one folder per program, one file per language, shared tests), a script that verifies every version against the tests, and the report.

## Threats to validity

- Who writes the versions matters: a skilled Python author writes shorter Python. Every version is reviewed for idiomatic, not golfed, code.
- Small programs exaggerate fixed costs such as `use` lines and `main`.

## Amendments

None.
