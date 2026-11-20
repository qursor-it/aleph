---
id: EXP-004
title: Fine-tuning and RL on a verifiable language
status: approved
question: Does fine-tuning plus RL with verifier rewards make a small open-weights model better at Aleph than the same training makes it at Python?
owner: Bernardo Manfriani
updated: 2026-11-20
---

# EXP-004 · Fine-tuning and RL on a verifiable language

## Question

An open-weights model can be taught Aleph directly, removing the need for `llms.txt` and the lack of training data. The real question is not whether it gets better (it will) but whether Aleph's built-in verifier (checker, examples, capabilities, determinism) makes training more effective than the same training on Python with the same budget.

## Hypotheses

- **H4.1**: after supervised fine-tuning on a verified corpus, the model's pass@1 in Aleph on the EXP-002 suite matches or beats its pass@1 in Python after the same fine-tuning.
- **H4.2**: RL with rewards from the Aleph verifier improves pass@1 more per training step than RL with rewards from Python tests.
- **H4.3**: the fine-tuned model no longer needs `llms.txt`, so its tokens per solved task are below both its own Python and the base model with the spec.

## Setup

- **Base model**: one open-weights code model of 7–8B parameters, the same one used in EXP-002.
- **Data**: the Orbis corpus, generated from task statements: verified Aleph programs plus repair triples (broken program, JSON diagnostics, fixed program). A Python corpus of the same size, from the same statements, verified by tests. No task of the EXP-002 suite or of HumanEval-Aleph, and no near-duplicate of one, enters training data (HumanEval-Aleph is derived from MultiPL-E, whose licence forbids using it as training data).
- **Training**:
  1. supervised fine-tuning on each corpus, same number of steps;
  2. RL from the fine-tuned checkpoint (a group-relative policy-gradient method), same number of steps and samples. Reward: hidden tests passed, plus `aleph check` clean and no capability violation for Aleph; tests passed and no file access outside the declared set (EXP-002 sandbox) for Python.
- **Evaluation**: the EXP-002 suite and harness, conditions without `llms.txt` for fine-tuned models.

## Baselines

- Base model with `llms.txt` (from EXP-002).
- Fine-tuned only (no RL), Aleph and Python.
- Fine-tuning plus RL on Python with the same budget.

## Metrics

pass@1, pass@5, rounds to pass, tokens per solved task, training curves (pass@1 every N steps), compute used (GPU hours).

## Analysis

As EXP-002 for evaluation. Training curves compared at equal steps; area under the curve as the efficiency measure. Three training seeds per arm if the budget allows, otherwise one, stated as such.

## Success criterion and decision

- H4.2 holds if the Aleph RL curve is above the Python one at equal steps across seeds, beyond seed-to-seed variation.
- If Aleph does not beat Python after training, the result is reported as it is: the verifier is not enough of an advantage at this scale.

## Cost and limits

Needs GPUs: budget and provider set by the person who starts the run (see Amendments). Started by a person, never by the agent loop.

## Tools to build

- **T-041**: Orbis generator (task statements to verified programs and repair triples), with deduplication against the EXP-002 suite.
- **T-042**: reward function wrapping `aleph check`, the examples and hidden tests, with a time limit, usable from an RL trainer.

## Threats to validity

- Reward hacking: the model could write trivial `ex` lines. Rewards come from hidden tests, never from the model's own examples.
- Leakage between training and evaluation tasks.
- One model size; results may not transfer to larger models.

## Amendments

- **2026-11-20, proposed by agent; approved under the owner's delegation of 20 Nov 2026** (the owner asked the agent to bring experiment protocols to `approved` before building their tools). Hypotheses, Metrics and Success criterion are unchanged. Clarifications needed to build T-041 and T-042:
  - **GPU budget.** `[GPU BUDGET TO SET]` is resolved as: budget and provider are chosen by the person who starts the training run and recorded in the run notes. The agent builds no training job and starts none.
  - **Orbis generator (T-041).** Programs come from a model behind the same backend interface as the EXP-002 harness (a fake model in tests); every kept program passes `aleph check` and its tests, repair triples pair a failing attempt, its JSON diagnostics and the fixed program. The agent runs the generator only with the fake model. Deduplication compares task statements (normalised word n-grams) against the EXP-002 suite and the HumanEval-Aleph prompts; it never reads hidden tests.
  - **Reward (T-042).** A pure function of a program and a task folder: hidden tests passed (fraction), plus `aleph check` clean and no capability violation for Aleph, with a per-program time limit; the model's own `ex` lines never add reward.
