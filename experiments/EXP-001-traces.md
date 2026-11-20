---
id: EXP-001
title: Where an agent's tokens go
status: approved
question: What share of a coding agent's tokens is code, and so what is the most a more compact language could save?
owner: Bernardo Manfriani
updated: 2026-11-20
---

# EXP-001 · Where an agent's tokens go

## Question

In real coding-agent sessions, how are tokens split between reasoning, instructions and context, code the agent writes, code it reads, and other tool output? Aleph can only shrink the code part, so this split puts an upper bound on any token saving. The experiment needs no Aleph at all, and it comes first.

## Hypotheses

- **H1.1**: code the agent writes is under 15% of all tokens in a typical session.
- **H1.2**: code the agent reads (file contents in tool results) is a larger share than code it writes.
- **H1.3**: the split depends on the kind of task (small script vs change in an existing repository).

## Setup

- **Traces**: at least 200 sessions of agents solving Python or bash tasks, from two sources:
  1. sessions of our own agents (opencode and others), recorded with the owner's consent and with secrets removed;
  2. publicly released agent trajectories whose licence allows analysis (to be selected and listed here before approval).
- **Task types**: at least three: one-file scripts, data and text processing, changes inside an existing repository.
- **Classification**: every span of every message gets one class: `system` (system prompt, tool definitions, specs), `user`, `reasoning`, `code_written` (code in tool calls that write or run files, or in code blocks), `code_read` (file contents returned by tools), `tool_other` (logs, errors, listings), `prose` (other assistant text).
- **Counting**: real tokenizers of the models that produced the traces where public; otherwise `o200k_base` as the reference. Input tokens served from a prompt cache are counted apart, when the trace says so.

## Baselines

None: this is a measurement, not a comparison.

## Metrics

- Share of tokens per class, per session and pooled, with 95% bootstrap intervals.
- The same by task type and by session length.
- **Upper bound on saving** `S_max = c_w · (1 − r) + c_r · (1 − r)`, where `c_w` and `c_r` are the shares of code written and read and `r` is the Aleph/Python size ratio measured in EXP-003 (until then, report `S_max` for r = 0.5, 0.7 and 0.9).

## Analysis

Descriptive statistics with bootstrap intervals, by source and task type. The classifier is checked by hand on 20 random sessions; agreement under 95% of tokens means fixing the classifier before reporting.

## Success criterion and decision

The experiment succeeds when the split is measured on at least 200 sessions and 3 task types with the hand check passed.

- If code (written + read) is under 20% of tokens, "fewer tokens" stops being Aleph's main claim: the project argues from correctness and containment, and EXP-003 is reduced to a size comparison.
- If it is over 40%, EXP-003 and the token part of EXP-002 get priority.

## Cost and limits

No model calls: it only reads existing traces. No GPU.

## Tools to build

- **T-035**: trace importer for at least two formats into one JSONL schema, span classifier, token counter, report.

## Threats to validity

- Our own sessions may not be typical; public trajectories may come from older agents.
- Hidden reasoning is not always in the trace: sessions without visible reasoning are reported apart, not mixed in.
- The classifier can confuse code inside prose with prose.

## Amendments

- **2026-11-20, proposed by agent; approved under the owner's delegation of 20 Nov 2026** (the owner asked the agent to bring experiment protocols to `approved` before building their tools). Hypotheses, Metrics and Success criterion are unchanged. Clarifications needed to build T-035:
  - Trace formats supported by the importer: (1) the opencode JSON event stream (`opencode run --format json`, as read by `pilot/probe/probe_opencode.py`); (2) Claude Code session JSONL (one message per line, content blocks `text`, `thinking`, `tool_use`, `tool_result`). Any other source is converted to the importer's own JSONL schema by an adapter added when the source is chosen.
  - Public trajectories are chosen by the person who runs the experiment and listed in the run notes before the run; the agent does not search for or download them (no network).
  - Tokenizers are read from local vocabulary files given by path (tiktoken format, e.g. `o200k_base.tiktoken`); nothing is downloaded. Without a vocabulary the report says so and uses a labelled `chars/4` estimate, never presented as a real count.
  - The tool reads only the files it is given. Collecting traces, removing secrets and the hand check of the classifier on 20 sessions are done by people.
