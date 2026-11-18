---
id: EXP-004
title: Fine-tuning and RL on a verifiable language
status: draft
question: Does fine-tuning plus RL with verifier rewards make a small open-weights model better at Aleph than the same training makes it at Python?
owner: Bernardo Manfriani
updated: 2026-11-18
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
- **Data**: the Orbis corpus, generated from task statements: verified Aleph programs plus repair triples (broken program, JSON diagnostics, fixed program). A Python corpus of the same size, from the same statements, verified by tests. No task of the EXP-002 suite, and no near-duplicate of one, enters training data.
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

Needs GPUs: budget and provider set at approval ([GPU BUDGET TO SET]). Started by a person, never by the agent loop.

## Tools to build

- **T-041**: Orbis generator (task statements to verified programs and repair triples), with deduplication against the EXP-002 suite.
- **T-042**: reward function wrapping `aleph check`, the examples and hidden tests, with a time limit, usable from an RL trainer.

## Threats to validity

- Reward hacking: the model could write trivial `ex` lines. Rewards come from hidden tests, never from the model's own examples.
- Leakage between training and evaluation tasks.
- One model size; results may not transfer to larger models.

## Amendments

None.
