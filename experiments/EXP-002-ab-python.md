---
id: EXP-002
title: Aleph against Python and bash
status: draft
question: How much does an agent's success rate, number of attempts and total cost change when it writes Aleph instead of Python or bash?
owner: Bernardo Manfriani
updated: 2026-11-18
---

# EXP-002 · Aleph against Python and bash

## Question

Given the same task, model and budget, how does an agent writing Aleph compare with the same agent writing Python or bash, in success rate, attempts, tokens, cost and safety? Aleph starts with a handicap (no training data, a spec to load), so the first question is how much is lost, then whether diagnostics and examples win it back.

## Hypotheses

- **H2.1 (non-inferiority)**: with `llms.txt` in context, pass@1 in Aleph is at most 10 points below Python.
- **H2.2**: Aleph reaches a passing program in no more rounds than Python, thanks to JSON diagnostics.
- **H2.3**: total tokens per solved task, counting the spec and every retry, are lower in Aleph. Cached input is reported apart.
- **H2.5 (direct or translated)**: when writing Aleph, the agent reasons in Aleph or in plain language, not by first drafting the program in Python and translating it. If it translates, its reasoning holds two programs and the token saving of H2.3 shrinks or turns negative.
- **H2.4**: Aleph programs never touch files outside their declared capabilities; Python and bash programs are checked for the same behaviour with a sandbox that logs file access.

## Setup

- **Language version**: a frozen tag, `v0.2-experiment` ("experiment-ready": dict, JSON, sum types with match, and the rest of the v0.2 list). The version does not change during the run.
- **Task suite**: 50 tasks in Aleph's scope: text, files, data, JSON, small algorithms. Each task has a natural-language statement, input files, hidden tests (inputs and expected output) and a declared set of files it may read and write. Tasks are written before any agent is run and are not tuned to any language.
- **Public benchmark**: the same conditions also run on HumanEval translated into Aleph (`experiments/benchmarks/humaneval/`, 156 of 161 problems, method of MultiPL-E), so Aleph can be compared with the results MultiPL-E has published for other languages on the same problems. Python and bash use MultiPL-E's own translations.
- **Conditions**:
  1. Python;
  2. bash (only for the tasks where bash is a natural choice, marked in advance);
  3. Aleph with `llms.txt`;
  4. Aleph with `llms.txt` plus 5 solved example tasks not in the suite.
- **Agent**: one simple, fixed harness for every condition: write the program, run the checker, read feedback, retry, up to 5 rounds. Feedback is `aleph check --json` and test results for Aleph; syntax check, linter and test results for Python and bash.
- **Models**: at least two: one frontier model via API and one open-weights model (the same base later used in EXP-004).
- **Repetitions**: 3 runs per task, condition and model, with fixed seeds where the API allows.

## Baselines

Conditions 1 and 2 are the baselines.

## Metrics

- pass@1 and pass@5 on hidden tests.
- Rounds to the first passing program.
- Tokens per solved task, split into new input, cached input, output and reasoning (counted as in EXP-001).
- Cost in euro per solved task at list prices on the date of the run.
- Wall-clock time per task.
- Safety: attempted accesses outside the declared files, per condition.
- **Translation rate** (H2.5): share of Aleph sessions whose visible reasoning contains a draft of the solution in another programming language, and the tokens it takes. Detected automatically (code blocks or lines that parse as Python, bash or another language) and checked by hand on 30 random sessions. Measured only on models that expose their reasoning; for the others it is reported as not observable.
- **Structural similarity** (H2.5, indirect): for each task, how close the Aleph solution is to the same model's Python solution (same decomposition into functions, same order of steps), compared with how close two independent Python solutions are. Much higher similarity suggests translation even when the reasoning is hidden.

## Analysis

Paired by task. McNemar's test for pass@1 between conditions; bootstrap intervals for rounds, tokens and cost; results by task category. Failures are classified by hand on a random sample of 30 per condition (did not understand the task / language error / logic error / tooling).

## Success criterion and decision

- H2.1 holds if the upper end of the 95% interval of the pass@1 gap is under 10 points.
- If Aleph loses more than 20 points even with examples (condition 4), what is missing is examined before EXP-004: language features, spec, or diagnostics.
- H2.3 is judged together with EXP-001: a saving is relevant only as a share of the whole session.

## Cost and limits

API calls cost money: the runner stops at a spending cap set at approval ([BUDGET TO SET]). It runs only when a person starts it, never inside the agent loop.

## Tools to build

- **T-036**: task suite format, 5 sample tasks, validator (statement, inputs, hidden tests, declared files).
- **T-037**: harness running one condition on one task with the round loop, the feedback for each language, token and cost logging, and the spending cap.
- **T-038**: sandbox that logs file access for Python and bash programs.

## Threats to validity

- Task selection can favour one language: tasks are written before running, by someone who is not tuning the harness, and published with the results.
- Models may have seen similar Python tasks in training.
- Prompts differ between languages by necessity: every prompt is published.
- Visible reasoning may not show everything the model does internally: H2.5 can show that translation happens, never prove that it does not.

## Amendments

None.
