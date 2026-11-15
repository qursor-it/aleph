# PROGRESS

Append-only log written by agents. Newest entries at the bottom. Format in AGENTS.md.

## 2026-11-12T09:45 T-000 done
- repository initialized with spec v0.1, ADR 0001–0014, TASKS, loop scripts
- tests: none yet
- next: start T-001

## 2026-11-15T10:15 T-001 done
- root workspace `Cargo.toml`; crate `compiler/` (package `alephc`, binary `aleph`, edition 2021, serde + serde_json)
- CLI skeleton for SPEC §11 subcommands: recognized commands emit an X001 JSON diagnostic and exit 1; options (`--json`, `-w`, `-o OUT`) are parsed separately from FILE; `run` passes trailing arguments through; unknown command/option or missing FILE prints usage to stderr and exits 2
- `src/lib.rs` exposes `cli` and `diag`; `src/cli.rs` parses `args_os` lossily; minimal SPEC §9 diagnostic struct in `compiler/src/diag.rs`; `scripts/check.sh` (fmt, clippy, test); CLI integration and parser unit tests
- tests: 14 passed
- decisions: ADR-0015 (Proposed, `X` prefix for tool/internal diagnostics)
- agent: opencode/deepseek-v4.1-flash
- next: T-002 expands the diagnostic struct to the full SPEC §9 fields and adds `spec/diagnostics.md`
