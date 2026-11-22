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
- [x] T-045 Lambdas, syntax: keyword `lam` in lexer/lexicon, parser (binder count from the higher-order builtin's function parameter), AST node with ids, printer canonical form, round-trip tests; P-error outside a function-argument position — ADR-0037
- [x] T-046 Lambdas, checker: check non-function arguments first, bind lambda binders from the instantiated signature, body type rules for map/filter/fold/map2, T005 for binders, captures of params/lets/match binders/outer binders — ADR-0037
- [x] T-047 Lambdas, interpreter: function values with captured values; goldens for map/filter/fold/map2 with captures, nested lambdas, traps inside lambdas (R001/R002/R004) — ADR-0037
- [x] T-048 Lambdas, native: `AlFn` with a captured-values array, lifted static C functions, callback signature passes the `AlFn`; update wrappers and `tlon/test_tlon.c`; differential tests byte-identical incl. traps — ADR-0037
- [x] T-049 Lambdas, tooling: `aleph explain` text and JSON, `aleph edit` on lambda nodes, MCP, `llms.txt` (budget amendment in ADR-0026 if needed) — ADR-0037
- [x] T-050 Recount the HumanEval part of the token corpus with lambdas on compiled, test-passing Aleph code and compare with the ADR-0037 estimate (median 0.81 Aleph/Python); report in experiments/ — ADR-0037
- [x] T-051 Generated C clean under `-Wall -Wextra`: unused `L_epilogue` labels in slow bodies, unused `L_n` location constants, set-but-unused temporaries in `main`; then build goldens with those flags in the differential test
- [x] T-052 Interpreter: compute each lambda's capture names once (when the `lams` table is built) and avoid cloning the captured env per call; keep traps and depth identical
- [x] T-044 Builtins `ord` (str -> opt i64, code of the first Unicode scalar; `none` on empty) and `chr` (i64 -> opt str, `none` outside valid scalars), in interpreter, Tlön and llms.txt, with goldens and differential tests. Found by the 18 Nov pilot: without them a model rebuilt character codes from an alphabet string and failed (HumanEval 66). Proposed ADR first (ADR-0035)
- [x] T-030 Scoped regions in Tlön — ADR-0007, ADR-0039 (resumed by the project owner on 20 Nov 2026: "complete the open tasks in order")
- [x] T-031 `net.host` + `fetch`, `time`, `env` capabilities — ADR-0038 (resumed by the project owner on 19 Nov 2026 after reviewing the pilot)
- [x] T-053 Regions for lambda bodies: mark/reset in the lambda code as for user functions (safe: map/filter/fold/map2 store a result after the lambda returns), so `fold lam ...` in `main` reclaims temporaries — ADR-0039, found by the T-030 review
- [x] T-034 Generated C: user fn names share the `aleph_fn_*` prefix with Tlön (`fn kind` clashes with `aleph_fn_kind` in tlon.h, X004); use a distinct prefix or rename the runtime symbol

## Experiment-ready (freeze for experiments)

- [x] T-043 Checklist for the `v0.2-experiment` freeze in `experiments/FREEZE.md`: which v0.2 features are in (dict, JSON, sum types + match, T-030, T-031, T-034), what is out, and the commands that must pass. A person creates the tag; the agent only prepares the checklist.

## Experiment tools (only when the named protocol is `approved`; see AGENTS.md, Experiments)

- [x] T-035 Trace importer (two formats → one JSONL schema), span classifier, token counter and report — EXP-001
- [x] T-036 Task suite format, 5 sample tasks and validator (statement, inputs, hidden tests, declared files) — EXP-002
- [x] T-037 A/B harness: one condition × one task, round loop with per-language feedback, token and cost log, spending cap; offline tests with a fake model — EXP-002
- [x] T-038 File-access logging sandbox for Python and bash programs — EXP-002
- [x] T-039 BPE counter in `aleph tokens` reading a local vocabulary file given by path (follow-up to ADR-0031) — EXP-003
- [x] T-040 Corpus layout (one folder per program, one file per language, shared tests), verification script and report — EXP-003
- [x] T-041 Orbis generator: task statements → verified programs and repair triples, deduplicated against the EXP-002 suite — EXP-004
- [x] T-042 RL reward function wrapping `aleph check`, examples and hidden tests, with a time limit — EXP-004

## Autonomous (owner mandate of 20 Nov 2026)

Goal: make Aleph the cheapest language for agents to read and write correctly. Metrics: tokens vs Python on verified programs (`experiments/tools/corpus` report), problems solved by a model writing Aleph (EXP-002 harness, run by a person), compile errors per problem (replay of recorded model answers). Tasks below are chosen by the agent ("autonoma"); each decision is logged in PROGRESS.md.

- [x] T-054 (autonoma) Replay metric: `experiments/tools/replay/` re-checks every recorded model answer (pilot probe runs, harness logs) with the current compiler, raw and after `aleph fmt`, and reports check-clean rate and diagnostic codes per condition; baseline logged. Offline, never reads hidden tests
- [x] T-055 (autonoma) Formatting left to the toolchain: `check`/`run`/`test`/`build` accept any layout that parses (F001 becomes a warning with the canonical text; `--strict` keeps it an error for CI and the goldens); llms.txt drops the layout rules it no longer needs. Proposed ADR (refines ADR-0003; text stays canonical through `aleph fmt`)
- [x] T-056 (autonoma) `[]` as an empty list when the expected type is known (`empty T` stays valid and canonical printing keeps `empty T`); pilot answers failed with P001 "empty list". Proposed ADR
- [x] T-057 (autonoma) `sortby f xs` (key function, stable, keys i64/f64/str): ranking tasks needed zero-padded complemented string keys (EXP-002 sample text-top-words). Proposed ADR, both engines, llms.txt within budget
- [x] T-059 `build_deep_list` (compiler/tests/cli.rs, 3000-item list literal in a 5000-deep recursion): two `cargo test` runs hung 4h40 in `cc1 -O2` on 20 Nov while the VPS was overloaded; not reproduced afterwards (gcc 1.5-2.9 s). Give native builds in tests a timeout (kill cc, fail with a clear message), measure gcc time/memory on the generated C, and shrink the generated code if it is the cause
- [x] T-058 (autonoma) Inline rule-3 fast path in `aleph_region_exit` (root in the head chunk past the mark and region < 256 KiB: return without the call); T-053 review measured ~13% on hot `map lam`
- [x] T-060 (autonoma) `mapr f xs` (f: A -> res B E; result res (list B) E, first error wins, both engines): the 4 worst corpus programs (histogram 2.77, maxmin 2.56, total 2.21, csv 3.02 Aleph/Python chars4) all spell "parse every string, stop at the first error" as a fold over a `res` accumulator with two helper functions. Proposed ADR; `@int` must be usable as f if builtins are allowed as fn refs
- [x] T-061 (autonoma) `repeat s n -> str` and `index xs x -> opt i64`: histogram builds `"*" * n` with join/map/range and a helper; csv finds a column with a recursive helper. Proposed ADR, llms.txt within budget
- [x] T-062 (autonoma) Compact C for literal lists: a list literal whose items are i64/f64/bool literals becomes one static table plus one depth check at the first item (same R004 location), instead of a temporary and an eval_enter/leave per item. The build_deep_list program (3000 items) generates 916 KB of C; gcc -O2 needs 47 s and 646 MB (T-059 measurement), which under memory pressure stalled the VPS for hours. Done for i64 and bool; f64 lists keep the old path
- [x] T-063 (autonoma) Rewrite the Aleph versions of the EXP-003 corpus idiomatically with today's language (lambdas, `[]`, mapr/sortby/repeat/index, `join [..] ""` for string building), not golfed: several bench programs predate lambdas and carry one-line helpers (`matches`, `keep`, `nonempty`, `number`). Every version keeps passing `corpus.py verify`; ratios before/after logged
- [x] T-064 (autonoma) `sqrt x -> f64` builtin (f64 -> f64, NaN for negative input as IEEE): the points program carries a 40-step Newton loop because Aleph has no square root. Proposed ADR, both engines identical (C `sqrt` is correctly rounded, Rust `f64::sqrt` too), llms.txt within budget

- [x] T-065 (autonoma) Guard lines: a non-last body line `if C E` (two arguments) returns `E` when `C` holds, else the body goes on. The replay's most common remaining error (7 of 12 diagnostics, P002 on Python-style early returns in fewshot `unique` and lite `x_or_y`). Parser, checker, both engines, printer, explain, SPEC, grammar, llms.txt within budget, goldens — ADR-0044
- [x] T-066 (autonoma) `cat` on lists: `cat T,T->T` with T `str` or `list U` (dev probe run1: `rotate` wrote `cat` on lists, `merge` built a reversed list with `push`). Both engines, llms.txt within budget, goldens — ADR-0045
- [x] T-067 (autonoma) Builtin `tof i64->f64` (dev probe run1 wrote this helper through `float cat show n ".0"` twice and named it `tof`; 5 HumanEval headers mix i64/f64). Both engines, reserved name, llms.txt within budget — ADR-0046
- [x] T-068 (autonoma) llms.txt example of idiomatic loops: replace the `run_eprint` example (stderr ordering, which no probe task needed) with a golden showing `filter`/`map`/`fold` + `lam` over a list and over `range 0 len xs`, a guard, `mapr`, `cat` on lists; keep 10 examples and the budget. Measure with dev probe run3 on tasks2 (size and helper count vs run2)
- [x] T-069 (autonoma) llms.txt: `push` signature says "(at end)": three dev probe answers built lists reversed with `push REC x`. Measured with runs 4-5 (isolated workspace from run5 on)
- [x] T-070 (autonoma) Corpus refresh with ADR-0044/0045 (guards, `cat` on lists): points (dead `addf` helper and the recursive perimeter loop -> `map2` over the rotated list), maxmin (guard instead of the `emit` helper), gcd, files-copy-prefix, text-top-words; idiomatic, not golfed; corpus verify passes
- [x] T-071 (autonoma) Builtin `fields str->list str` (split on runs of ASCII whitespace, no empty items; Python `split()`, Go `strings.Fields`): text-top-words and the word-counting probe tasks spell it as a filter over `split s " "`. Both engines, reserved name, HumanEval translator reserved list updated, llms.txt within budget (shorten section 8) — ADR-0047
- [x] T-072 (autonoma) Builtin `okor x e` (`opt T`/`res T U`, E -> `res T E`; Rust `ok_or`/`map_err`): dev probe answers test-then-unwrap (`if has X ok else X d err e`), parse twice, and two hidden-test failures came from `try` passing the builtin's own error message. Both engines, reserved, llms.txt within budget, corpus json-field — ADR-0048
- [x] T-073 (autonoma) `E002` names a list found reversed (`push` built front-first): the commonest wrong answer in dev probe runs 1, 3, 5, 8; message only, no llms.txt cost; golden ex_e002_reversed — ADR-0049
- [x] T-074 (autonoma) llms.txt prose trim without content loss for writing code (§9 diagnostics shape, §10 canonical form, §4 "prefix, fixed arity", §11 `llms`, RFC 8259, `sort` ascending, §2 duplicates): 8433 -> 8212 chars; dev probe run9 (tasks4) 19/20 with the trimmed text
- [x] T-075 (autonoma) Corpus EXP-003 rewrites with okor/inline single-use helpers/`do try write`: data-csv-column-sum, text-top-words, gcd, json-field, files-copy-prefix; corpus verify passes
- [x] T-076 (autonoma) T001 on `len STR` / `count LIST` names the other builtin, no misleading `show` fix (dev probe run9 `wrap`) — ADR-0050
- [x] T-077 (autonoma) llms.txt `total` example uses `okor` and `fields`; a "prefer map/fold" line was measured (dev probe run10, run11) and dropped; new task set 5 (run12, run12-haiku)
