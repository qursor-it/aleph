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

## 2026-11-15T11:35 T-005 done
- `compiler/src/ast.rs`: node structs (`File`, `Use`, `TypeDef`, `Field`, `Fn`, `Param`, `Example`, `Stmt`, `Type`, `Expr`) with `NodeId`/`Span`; kind enums for statements, types, expressions, binops, unops, capabilities and callee kinds
- `assign_ids(&mut File)` assigns deterministic pre-order ids (`n0..`) across uses, typedefs/fields/types, fn params/ret/examples/stmts/result; `File::canonicalize` sorts uses by canonical line text, types by name and fns by name with `main` last; `File::same_ids` plus hand-written `PartialEq` ignoring ids/spans
- JSON via typed `Serialize` only: span flattened as `line`/`col`, kind enums tagged `kind` + flattened fields, `doc` always present; `to_json` is pretty JSON plus trailing newline
- operator and capability spellings come from `lexicon` (no hard-coded keyword strings); struct-like expression/type variants are used because internally tagged enums have no tuple-variant representation
- tests: 5 AST tests, including SPEC §12 `add` ids n0..n14, an exact JSON golden, and a nested `do` statement; `aleph ast` CLI wiring is left for T-007
- decisions: none (implements ADR-0003); noted struct-variant JSON choice above
- next: T-006 parser pass 1 can build the AST directly
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T11:40 T-011 done
- `compiler/src/builtins.rs`: `Ty`, `TyVar`, `Effect`, `BuiltinSpec` (`name`, `params`, `ret`, `effect`, `capability`, `path_arg`, `arity()`), `BUILTINS` sorted by name and `lookup(name)`
- all 27 SPEC §5.1 builtins, pure and effectful; `read`/`write` are effectful with `FsRead`/`FsWrite` and `path_arg: Some(0)`; `print`/`eprint` are effectful with no capability; `sort` encodes its restriction as `list Primitive`
- tests: names match `lexicon::builtin_names` exactly (and are sorted), every arity from SPEC §5.1, effects/capabilities for read/write/print/eprint; `nil`/`empty`/`none` are not builtins
- tests: 59 passed (48 unit, 10 CLI, 1 golden)
- decisions: none
- next: T-006 parser pass 1; use `builtins::lookup` for call arity
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T11:55 T-005/T-011 review fixes done
- `builtins`: `BuiltinSpec.bounds: &[(TyVar, Bound)]` with `Bound::{Primitive,Showable}`; removed `Ty::Primitive`/`Ty::Showable`. `sort` is now `list T -> list T` with `[(T, Primitive)]`, `show` is `T -> str` with `[(T, Showable)]`; test checks both share `TyVar::T` and have the right bound
- `ast`: public `escape_str` (only `\n`, `\t`, `"`, `\`); `Use::canonical_text` escapes the quoted path before sorting; test covers a path with quote, newline, tab and backslash
- `ast`: `File::finish` runs `canonicalize` then `assign_ids` (documented on all three); `to_json` debug-asserts no `NodeId::UNASSIGNED`; test checks finish produces ids 0..n in collect order
- `ast`: comment above every manual `PartialEq` impl reminding to update it when adding node fields
- ADR-0017 (Proposed): float literals preserve token text verbatim, so `1.0` and `1.00` are distinct canonical texts; normalization deferred until an Aleph-native format exists
- tests: 62 passed (51 unit, 10 CLI, 1 golden)
- decisions: ADR-0017 (Proposed)
- next: T-006 parser pass 1; float parsing must keep raw token text
- agent: opencode/deepseek-v4.1-flash
