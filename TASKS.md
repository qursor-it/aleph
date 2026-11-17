# TASKS

Format: `- [ ] T-NNN title — refs` . Mark done with `- [x]`. Blocked: append `BLOCKED: reason`.
Work top to bottom. Add new tasks with the next free id where they belong.

## M1 — Core (lexer → parser → printer → checker → interpreter)

- [x] T-001 Cargo crate `compiler/` (alephc, bin `aleph`), CLI skeleton with subcommands from SPEC §11 returning "not implemented" diagnostics; CI script `scripts/check.sh` running fmt, clippy, test — SPEC §11, ADR-0002
- [x] T-002 Diagnostics module: struct + JSON output per SPEC §9; `spec/diagnostics.md` table — ADR-0011
- [x] T-003 Lexicon module mirroring `spec/lexicon.toml` + test that they match — ADR-0005
- [x] T-004 Lexer: line structure, token classes, L001/L002, golden tests — SPEC §1–2
- [x] T-005 AST types with node ids (pre-order), `aleph ast` JSON output — ADR-0003 (`aleph ast` wired in T-007)
- [x] T-006 Parser pass 1: collect use/type/fn headers and signatures (types parsed prefix) — SPEC §3–4
- [x] T-007 Parser pass 2: arity-directed expressions, body lines (doc/ex/let/do/result), P001–P003 — SPEC §5, ADR-0004
- [x] T-008 Printer: canonical form; round-trip property tests; `aleph fmt` — SPEC §10
- [x] T-009 Canonical check F001 with `canonical` field in diagnostic — SPEC §10
- [x] T-010 Type checker: primitives, lists, opt/res, records, bidirectional ok/err/none, T001/T005/T010 — SPEC §4–5
- [x] T-011 Builtin table (types, arity, effect, capability) shared by parser, checker, interpreter — SPEC §5.1
- [x] T-012 Capability checker C001/C002/C003 — SPEC §8, ADR-0008, ADR-0020
- [x] T-013 Interpreter: all expressions and builtins, traps R001–R004 with JSON + exit 70 — SPEC §6, ADR-0009
- [x] T-014 Examples runner: E001/E002, `aleph test`, run inside `aleph check` — ADR-0010, ADR-0022
- [x] T-015 Golden test harness: one `compiler/tests/golden.rs` for `.out`/`.diag`/`.ast`/`.fmt`, orphans rejected, hermetic run dirs; SPEC §12 examples + 15 benchmark programs
- [x] T-016 `aleph explain` (deterministic): capabilities, signatures, docs, examples, call graph; text and `--json`

## M2 — Native (C backend + Tlön runtime)

- [x] T-017 Tlön runtime in C: arena allocator, strings, lists, checked arithmetic, traps, capability checks — ADR-0006, ADR-0007
- [x] T-018 C codegen for expressions and functions; `aleph build` invokes `cc` — ADR-0006
- [x] T-019 C codegen for all builtins incl. map/filter/fold via function pointers
- [x] T-020 Differential tests: interpreter vs native output identical for all golden programs
- [x] T-021 Benchmark harness `bench/run.sh`: time + memory of native Aleph vs Python vs C on `bench/tasks.md` programs (write reference versions)
- [x] T-032 Fix f64 `sort`: every NaN after `inf` whatever its sign bit (x86-64 vs arm64 differed) — ADR-0024
- [x] T-033 Fast path codegen: unboxed i64/f64/bool, no per-node depth accounting, exact R004 via slow fallback — ADR-0027

## M3 — Agent tooling

- [x] T-022 `llms.txt` at repo root: compact spec (target < 2000 tokens) + 10 canonical examples, generated from SPEC
- [x] T-023 `fix` suggestions in diagnostics for the 10 most common errors (arity, missing try, missing capability, non-canonical) — arity (P002) deliberately gets no fix, see ADR-0028
- [x] T-024 Node-level edit command `aleph edit FILE --json EDITS` (replace/insert/delete by node id) — ADR-0003
- [x] T-025 MCP server (stdio) exposing spec, check, run, test, fmt, explain, edit — design doc §5.5. Write a Proposed ADR for crate choice first.
- [x] T-026 Token counter `aleph tokens` with pluggable tokenizers (start with a local BPE file if provided; otherwise stub + ADR)

## v0.2 language (do not start before M2 is done)

- [x] T-027 `dict K V` type and builtins (Proposed ADR first)
- [x] T-028 JSON parse/print (Proposed ADR first)
- [x] T-029 Sum types + `match` (Proposed ADR first)
- [ ] T-030 Scoped regions in Tlön — ADR-0007
- [ ] T-031 `net.host` + `fetch`, `time`, `env` capabilities
- [ ] T-034 Generated C: user fn names share the `aleph_fn_*` prefix with Tlön (`fn kind` clashes with `aleph_fn_kind` in tlon.h, X004); use a distinct prefix or rename the runtime symbol
