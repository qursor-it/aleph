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

## 2026-11-15T10:24 T-002 done
- `compiler/src/diag.rs`: full SPEC §9 `Diagnostic` (required fields plus optional `node`, `expected`, `found`, `fix`, `canonical`), `Fix {op,node,with}`, `Diagnostic::error` + `with_node`/`with_expected_found`/`with_fix`/`with_canonical`, `to_json_line`, `emit_all`; X001 `not_implemented` unchanged
- `spec/diagnostics.md`: stable code table (SPEC §9 codes + X001), checked against the Rust `CODES` table by an `include_str!` test
- tests: 19 passed (5 new diag tests)
- decisions: none (implements ADR-0011 and ADR-0015)
- next: T-003 lexicon module
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T10:24 T-003 done
- `compiler/src/lexicon.rs`: constants `VERSION`, `KEYWORDS`, `TYPES`, `OPERATORS`, `BINARY_OPERATORS`, `PUNCTUATION`, `BUILTINS` mirroring `spec/lexicon.toml`; lookups `keyword`/`is_keyword`/`keyword_names`, `type_name`/`is_type_name`/`type_names`, `operator`/`is_operator`/`is_binary_operator`/`is_punctuation`/`operator_names`/`binary_operator_names`/`punctuation_names`, `builtin_names`/`is_builtin`
- test parses `spec/lexicon.toml` with a tiny line-based parser, rejects duplicate keys, checks exact key sets for every section, and checks every section both ways; SPEC §2 keyword list must all be reserved; `->`/`@`/`.` are punctuation, not operators
- tests: 23 passed (4 new lexicon tests)
- decisions: none
- next: T-004 lexer
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T10:48 T-004 done
- `compiler/src/lexer.rs`: `lex(file, source) -> (Vec<Line>, Vec<Diagnostic>)`; line structure with `TopLevel`/`Body`, 1-based byte columns; token classes Ident, Keyword, Int, Float, Str (decoded value + raw text), FnRef, Field, Param, LBracket/RBracket, Eq, Arrow, Op, CapName; L001/L002 collected without stopping; keyword/operator/arrow/fnref/field spellings come from `lexicon`; capability names recognized only after `use`; SPEC §12 examples lex cleanly
- whitespace is tolerant for `aleph fmt`: multiple/final spaces, empty lines, missing final LF and ≥1 leading space (Body) accepted; tabs/CR still L001 (ADR-0016 Proposed)
- `aleph check` now reads the file (X002 `cannot read file` on failure), emits lexer diagnostics as JSON lines and exits 1; on a lex-clean file it keeps the X001 not-implemented stub; other subcommands remain stubbed
- `spec/diagnostics.md` and `CODES` gain X002; golden cases in `tests/aph/*.aph` + `.diag`; `compiler/tests/lexer_diag.rs` runs `aleph check` over every `.aph` with a `.diag` (sorted, byte-for-byte stdout)
- tests: 48 passed (14 lexer unit, 2 new CLI, 1 golden harness; existing CLI tests updated for X002/check behavior)
- decisions: ADR-0016 (Proposed, lexer whitespace tolerance)
- next: T-005 AST with node ids; parser can consume `Vec<Line>`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T11:08 T-004 done
- review fixes: `lexicon::is_binop`/`is_unop` (symbolic ops plus `and`/`or`, `not`/`neg` via keyword lookup) and `lexicon::CAPABILITIES`/`is_capability`; lexer now consumes both
- `lexer::lex` documents that `lines` is meaningful only when diagnostics is empty; diagnostics are stable-sorted by `(line, col)`; bad escapes keep the raw backslash+char in the decoded string value
- ADR-0015 lists X002; ADR-0016 documents that spaces inside `x:T`, `@f`, `.f` are not tolerated (L002)
- lexer tests use `lexicon::keyword(...)` for keyword tokens instead of hard-coded spellings; added binop/unop grammar test, capability test and diagnostic-order/bad-escape tests
- tests: 51 passed (40 unit, 10 CLI, 1 golden)
- decisions: ADR-0015/0016 updated (no status change)
- next: T-005 AST with node ids
- agent: opencode/deepseek-v4.1-flash
