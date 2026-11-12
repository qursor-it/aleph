# TASKS

Format: `- [ ] T-NNN title — refs` . Mark done with `- [x]`. Blocked: append `BLOCKED: reason`.
Work top to bottom. Add new tasks with the next free id where they belong.

## M1 — Core (lexer → parser → printer → checker → interpreter)

- [ ] T-001 Cargo crate `compiler/` (alephc, bin `aleph`), CLI skeleton with subcommands from SPEC §11 returning "not implemented" diagnostics; CI script `scripts/check.sh` running fmt, clippy, test — SPEC §11, ADR-0002
- [ ] T-002 Diagnostics module: struct + JSON output per SPEC §9; `spec/diagnostics.md` table — ADR-0011
- [ ] T-003 Lexicon module mirroring `spec/lexicon.toml` + test that they match — ADR-0005
- [ ] T-004 Lexer: line structure, token classes, L001/L002, golden tests — SPEC §1–2
- [ ] T-005 AST types with node ids (pre-order), `aleph ast` JSON output — ADR-0003
- [ ] T-006 Parser pass 1: collect use/type/fn headers and signatures (types parsed prefix) — SPEC §3–4
- [ ] T-007 Parser pass 2: arity-directed expressions, body lines (doc/ex/let/do/result), P001–P003 — SPEC §5, ADR-0004
- [ ] T-008 Printer: canonical form; round-trip property tests; `aleph fmt` — SPEC §10
- [ ] T-009 Canonical check F001 with `canonical` field in diagnostic — SPEC §10
- [ ] T-010 Type checker: primitives, lists, opt/res, records, bidirectional ok/err/none, T001/T005/T010 — SPEC §4–5
- [ ] T-011 Builtin table (types, arity, effect, capability) shared by parser, checker, interpreter — SPEC §5.1
- [ ] T-012 Capability checker C001/C002 — SPEC §8, ADR-0008
- [ ] T-013 Interpreter: all expressions and builtins, traps R001–R003 with JSON + exit 70 — SPEC §6, ADR-0009
- [ ] T-014 Examples runner: E001/E002, `aleph test`, run inside `aleph check` — ADR-0010
- [ ] T-015 Golden test harness: every `tests/aph/*.aph` with `.out` or `.diag`; add the SPEC §12 examples + 15 more programs
- [ ] T-016 `aleph explain` (deterministic): capabilities, signatures, docs, examples, call graph; text and `--json`

## M2 — Native (C backend + Tlön runtime)

- [ ] T-017 Tlön runtime in C: arena allocator, strings, lists, checked arithmetic, traps, capability checks — ADR-0006, ADR-0007
- [ ] T-018 C codegen for expressions and functions; `aleph build` invokes `cc` — ADR-0006
- [ ] T-019 C codegen for all builtins incl. map/filter/fold via function pointers
- [ ] T-020 Differential tests: interpreter vs native output identical for all golden programs
- [ ] T-021 Benchmark harness `bench/run.sh`: time + memory of native Aleph vs Python vs C on `bench/tasks.md` programs (write reference versions)

## M3 — Agent tooling

- [ ] T-022 `llms.txt` at repo root: compact spec (target < 2000 tokens) + 10 canonical examples, generated from SPEC
- [ ] T-023 `fix` suggestions in diagnostics for the 10 most common errors (arity, missing try, missing capability, non-canonical)
- [ ] T-024 Node-level edit command `aleph edit FILE --json EDITS` (replace/insert/delete by node id) — ADR-0003
- [ ] T-025 MCP server (stdio) exposing spec, check, run, test, fmt, explain, edit — design doc §5.5. Write a Proposed ADR for crate choice first.
- [ ] T-026 Token counter `aleph tokens` with pluggable tokenizers (start with a local BPE file if provided; otherwise stub + ADR)

## v0.2 language (do not start before M2 is done)

- [ ] T-027 `dict K V` type and builtins (Proposed ADR first)
- [ ] T-028 JSON parse/print (Proposed ADR first)
- [ ] T-029 Sum types + `match` (Proposed ADR first)
- [ ] T-030 Scoped regions in Tlön — ADR-0007
- [ ] T-031 `net.host` + `fetch`, `time`, `env` capabilities
