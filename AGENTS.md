# AGENTS.md — rules for agents working on Aleph

You are building **Aleph**, a programming language for AI agents. Read this file fully at the start of every session. You have no memory between sessions: the only memory is the repository (`TASKS.md`, `PROGRESS.md`, `adr/`, git history).

## Sources of truth (in priority order)

1. `spec/SPEC.md` and `spec/grammar.ebnf` — what the language is.
2. `adr/*.md` with status Accepted — why, and constraints you must respect.
3. `TASKS.md` — what to do next.
4. `docs/` — background (design document PDF). Informational only.

If the spec is silent or ambiguous: make the smallest reasonable decision, write it as a new ADR with status `Proposed` (next free number), implement it, and note it in `PROGRESS.md`. Never silently change an Accepted ADR or the spec semantics; if you think one is wrong, write a Proposed ADR that supersedes it and continue with the current rule.

## Repository layout

```
compiler/      Rust crate `alephc`, binary `aleph`
  src/lexer.rs parser.rs ast.rs printer.rs check.rs interp.rs diag.rs cgen.rs explain.rs main.rs
  tests/       integration tests
tlon/          runtime: arena allocator + builtins in C (used by the C backend)
tests/aph/     golden programs: NAME.aph + NAME.out (expected stdout) or NAME.diag (expected JSON diagnostics)
bench/         benchmark tasks
corpus/        Orbis output (verified programs) — only in orbis mode
spec/ adr/ docs/ loop/
```

Create directories as tasks require. Keep the compiler a single Cargo crate until a task says otherwise.

## Engineering rules

- Language: Rust (stable), edition 2021. No dependencies except: `serde`, `serde_json` (diagnostics/AST JSON), and dev-dependencies `insta` or plain golden files. Ask (via a Proposed ADR) before adding any other crate.
- Tests first: for each feature add golden tests in `tests/aph/` and unit tests. A task is done only when its tests pass.
- Before every commit run, and fix until all pass:
  `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`.
- Never delete, skip or weaken a test to make it pass. If a test is wrong per the spec, fix the test and explain in the commit.
- Determinism: same input → byte-identical output (diagnostics, printer, codegen).
- Diagnostics are JSON objects per SPEC §9. No prose error messages to stdout.
- Every AST node has a stable id (`n0`, `n1`, … in pre-order) used by diagnostics and `aleph ast`.
- Keep functions small and code boring. Correctness over cleverness.

## Work loop (one session)

1. Read `AGENTS.md`, the last ~60 lines of `PROGRESS.md`, and `TASKS.md`.
2. Run `cargo test --all` (if the crate exists). If the tree is red, fixing it is the task.
3. Take the first task marked `- [ ]` that is not `BLOCKED`. Do one task (two if both are tiny).
4. Read the spec sections and ADRs that the task references.
5. Write tests, implement, run the checks.
6. Mark the task `- [x]` in `TASKS.md`. If you discovered new work, add tasks at the right place with the next free id.
7. Append an entry to `PROGRESS.md` (format below).
8. Commit (format below). Then stop the session.

If you fail on the same task for 2 sessions (check `PROGRESS.md`), mark it `BLOCKED: <reason>` in `TASKS.md`, log it, commit, and move to the next task.

## PROGRESS.md entry format

```
## 2026-11-12T03:14 T-007 done|partial|blocked
- what changed (files, features)
- tests: N passed
- decisions: ADR-00NN (if any)
- next: what the next session should know
```

## Commit format (agent-ready, ADR-0013)

```
<area>: <summary, imperative, max 60 chars>

Intent: <why, one line>
Task: T-007
Changed: <component/file: what>; <...>
Capabilities: none | <capability semantics changed>
Spec: unchanged | ADR-00NN proposed
Verified: cargo test (<N> passed), clippy clean
Agent: opencode/<model>
```

`<area>` is one of: lexer, parser, ast, printer, check, interp, cgen, tlon, cli, diag, explain, spec, adr, tests, bench, corpus, loop.

## Hard limits

- Never run `git push`, never change git remotes, never rewrite history (`reset --hard`, `rebase`, `commit --amend` on existing commits).
- Never access the network except through `cargo` for allowed crates.
- Never read or write outside this repository.
- Never touch `loop/`, `opencode.json` or `AGENTS.md` unless a task explicitly says so.
- If you create a file called `STOP` or find one, do nothing and end the session.
