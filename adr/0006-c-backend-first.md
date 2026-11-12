# ADR-0006: Interpreter plus C backend first; LLVM and WASM later

Status: Accepted
Date: 2026-11-12

## Context
Native speed is a goal, but writing a machine-code generator is costly. Agents also need instant feedback.

## Decision
- Reference interpreter (in Rust) for `run`, `test` and examples at check time.
- First native backend: generate C99, compile with the system `cc` (clang/gcc; tcc for speed when available), linking the Tlön runtime in `tlon/`.
- Differential testing: every golden program must produce identical output in the interpreter and the C build.
- LLVM backend (or Cranelift) and WASM/WASI target come after M2.

## Consequences
- No hand-written assembly.
- Generated C must be free of undefined behavior: checked arithmetic, bounds checks.
