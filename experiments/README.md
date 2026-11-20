# Experiments

Aleph makes claims that can be wrong: that agents write correct programs in it, that it saves tokens, and that a model trained on it gets better faster. Each claim is tested by an experiment written down **before** it runs.

| ID | Question | Status |
|---|---|---|
| [EXP-001](EXP-001-traces.md) | What share of an agent's tokens is code at all? | approved |
| [EXP-002](EXP-002-ab-python.md) | How much performance does an agent lose writing Aleph instead of Python or bash? | approved |
| [EXP-003](EXP-003-token-corpus.md) | How many tokens does the same verified program take in Aleph, Python, bash and C? | approved |
| [EXP-004](EXP-004-finetune-rl.md) | Does fine-tuning plus RL on Aleph beat the same training on Python? | draft |

Before the protocols are approved, sessions run by hand go in [`pilot/`](pilot/README.md), in a fixed format. They help to design the experiments and never count as evidence.

## Lifecycle

`draft` → `approved` → `running` → `done` (or `abandoned`, with the reason).

- **draft**: the protocol is being written and reviewed. Nothing is built.
- **approved**: a person has reviewed the protocol and changed the status. The loop agent may now build its tools (see `AGENTS.md`, Experiments).
- **running**: a person has started the run, with a spending cap.
- **done**: results are in `results/EXP-NNN.json` and summarised in the protocol.

Only people change the `status` field. Protocols change after approval only through a dated entry in their **Amendments** section, so the plan can always be compared with what was done.

## Every protocol has

Front matter (`id`, `title`, `status`, `question`, `owner`, `updated`), then: Question, Hypotheses, Setup, Baselines, Metrics, Analysis, Success criterion and decision, Cost and limits, Tools to build, Threats to validity, Amendments.

## Results

`results/EXP-NNN.json` follows `results/schema.json`: numbers, date, compiler commit, models and tokenizers used, and where the raw data lives. The website reads these files; a result that is not in a results file is not reported anywhere.
