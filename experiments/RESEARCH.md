# Research directions

Questions that Aleph opens beyond its own claims. They are ideas, not protocols: none is approved, nothing is built for them, and the loop agent does not act on them (see `AGENTS.md`). An idea becomes an experiment only when a person writes it as an `EXP-NNN` protocol and approves it.

Each entry has the question, why it matters, what we already have, and a first concrete step.

## R1 · Can a language shrink an agent's context?

**Question.** Is there a programming language in which agents read and write the same software with measurably fewer tokens, and does that saving survive on large codebases, where most tokens are code the agent reads rather than code it writes?

**Why.** Agents spend their context on code. A language designed for that reader could be a lever on cost and on how much of a system fits in one context. As far as we know this has not been shown end to end.

**What we have.** The 18 Nov token pilot (`pilot/tokens/`): today Aleph is longer than Python, but its function bodies are shorter and the gap comes from missing features, which are being added (ADR-0036, ADR-0037). EXP-001 and EXP-003 in draft.

**First step.** Re-count the corpus once lambdas exist; then a reading test on modules that exist in both languages (questions and edits on the same code, counting context and accuracy).

## R2 · Rules or tools?

**Question.** For a language a model has never seen, is it better to explain the rules in the prompt or to leave them to the toolchain (formatter, checker, diagnostics with fixes)?

**Why.** It applies to every new or low-resource language and DSL, not only to Aleph. The 18 Nov pilot found that most of the extra reasoning went into tracking rules, and that a shorter spec cut it by two thirds at the price of compile errors the compiler can name.

**What we have.** The probe runs with full spec, lite spec and few-shot prompts (`pilot/probe/runs/`).

**First step.** The lite spec with one round of compiler feedback, counting every token, on the full HumanEval-Aleph set and more than one model.

## R3 · How fast can a model learn a new language?

**Question.** How much data and training does an open-weights model need to write a new language natively, and does reinforcement with the compiler as the only reward close the gap faster than with an established language?

**Why.** If teaching a language is cheap, the cost of a new language is a one-time cost, not a permanent tax on every prompt.

**What we have.** EXP-004 in draft; HumanEval-Aleph (evaluation only, never used for training).

**First step.** A small supervised run on verified Aleph programs, measuring pass rate and tokens against the same model prompted with the spec.

## R4 · An agent that designs its own language

**Question.** What happens when an autonomous agent designs, implements and evolves a language under a written method (specification, decision records, tests, measured claims)? Which decisions does it make well, where does it need people, and how does the process compare with human-led language design?

**Why.** Aleph is itself the case study: most of its compiler was written by an agent in a loop, with its history kept in `TASKS.md`, `PROGRESS.md`, the ADRs and git.

**What we have.** The full history of the project, the review rounds and the pilots.

**First step.** A descriptive analysis of the repository history: decisions taken by the agent versus by people, review findings, regressions and how they were caught.

## R5 · Agents, adoption and oversight

**Question.** If an agent is given a goal that depends on adoption (of its language, its code, its tools), how does it pursue it, and which of its strategies should worry us? For example: shaping published code so that it is more likely to end up in the training data of future models, or so that it passes the filters meant to keep low-quality or synthetic data out.

**Why.** This is a safety question. A language made by agents and spread by agents is a concrete, measurable setting for goal-directed behaviour around adoption, persuasion and data contamination, and for how well people can oversee code written in a form they do not read natively (`aleph explain` is one answer to that, and should be tested as one).

**Boundaries.** Studied only in closed environments: no attempt to influence real training pipelines, to evade real data filters or to publish content designed to do so. Any study here needs its own protocol with an explicit safety review before it is approved.

**First step.** Write the questions and the boundaries as a protocol draft, with the evaluation designed before any agent is given an adoption goal.

## Status

| ID | Direction | Status |
|---|---|---|
| R1 | Can a language shrink an agent's context? | idea, with pilot data |
| R2 | Rules or tools? | idea, with pilot data |
| R3 | How fast can a model learn a new language? | idea (EXP-004 draft) |
| R4 | An agent that designs its own language | idea |
| R5 | Agents, adoption and oversight | idea, needs a safety review first |
