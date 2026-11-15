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

## 2026-11-15T11:42 T-006 done
- `compiler/src/parser.rs`: `parse(file, source) -> ParseOutput` lexes first (any L diagnostic returns no AST) and `parse_lines(file, &[Line])` for tests; `File::finish` canonicalizes and assigns ids
- pass 1 dispatches top-level lines on lexicon keywords: `use` (capability + optional decoded path), `type` (fields with prefix fixed-arity types), `fn` (params + return type, no trailing tokens); types support `i64/f64/bool/str/unit`, `list`/`opt` (1), `res` (2), other idents as `Named`
- per-file `Signatures` stores fn arity and record ctor arity; raw body lines are kept per fn name in `bodies` for pass 2 (T-007). Fn results are placeholder `nil` expressions until T-007 replaces them (documented)
- header errors emit one P001/P002 per line, skip the item and its body lines, and parsing continues; body lines before a fn or after use/type are P001; duplicate fn/type names are P001 "duplicate definition"
- `ast::Capability::from_spelling` added for the parser
- tests: 82 passed (71 unit, 10 CLI, 1 golden); 20 new parser tests incl. nested types `res list i64 str`, `list opt str`, records, each P001/P002 case and SPEC §12 signatures
- decisions: none (implements ADR-0004)
- next: T-007 pass 2 consumes `Signatures` + `bodies`, parses expressions, replaces the placeholder result, and wires `aleph ast`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T11:53 T-007 done
- `compiler/src/parser.rs`: pass 2 parses each fn body by first token (`doc`, `ex`, `let`, `do`, or the result), fills `doc`/`examples`/`stmts`/`result`, and replaces the pass-1 placeholder; expressions are arity-directed from pass-1 signatures and `builtins::lookup`
- all SPEC §5 expression forms: literals (int range checked, float text preserved), vars, fnrefs, calls (fn/ctor/builtin), binops/unops, if/else, some/ok/err/try/has/isok, empty/none with types, field, lists and nesting; line-level leftovers are P002 at fixed-arity roots and P001 at 0-arity roots; missing args are P002 with expected/found
- body rules: single `= ` split for `ex`, globals-only example environments, `let` value before binding, result then no further lines, missing result at end of line, duplicate/malformed doc; expression depth capped at 256 (P001 "expression too deep")
- shadowing per ADR-0018: param/let collisions with params/locals or callables are T005; the binding is not inserted, so callable arity stays available
- fn and type names share one namespace: a collision is P001 "duplicate definition"
- `aleph ast FILE` wired: X002 on read failure, parser diagnostics on stdout with exit 1, else pretty JSON and exit 0; `aleph check` now runs lex+parse before the X001 stub
- golden: `tests/aph/parse_p001|p002|p003|t005` with `.diag`, and `ast_add.aph`/`ast_add.ast` compared by the extended harness
- tests: 113 passed (97 unit, 14 CLI, 2 golden)
- decisions: ADR-0018 (Proposed, shadowing callables)
- next: T-008 printer/canonical form; T-009 F001; the parser now produces complete `File`s for both SPEC §12 programs
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T12:23 T-006/T-007 review fixes done
- parser: type parsing now carries a nesting depth and reports P001 "type too deep" past 256 (6000 nested `list` test); expression leftover handling distinguishes non-expression tokens (P001 at the leftover), fixed-arity roots (P002 at head, found = arity + greedily parseable extras) and zero-arity roots (P002 for 0-arity calls, P001 at the first extra for vars/literals)
- names: `fn`/`type` names matching a builtin or lexicon type are P001 "reserved name"; duplicate record fields are P001 "duplicate field"
- shadowing: ADR-0018 updated; T005 now inserts the colliding param/let into the environment after reporting (one T005 per declaration) so later uses resolve as variables instead of cascading P002
- parser recovery: `result_seen` is set only after a successful result line; a failed result line no longer makes the rest of the fn "line after result", and missing result is only reported when the last body line parsed cleanly
- dedupe: one `p002` helper; one `body_keyword` predicate; expression head arity comes from a single `head_arity`/`resolve_name` pair used by parsing and leftover checks
- tests: 118 passed (102 unit, 14 CLI, 2 golden); new tests for type depth, leftovers, reserved names, duplicate fields, T005 insertion/multiple params and failed-result recovery
- next: T-008 printer/canonical form; T-009 F001
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T12:30 T-008 done
- `compiler/src/printer.rs`: `print_file(&ast::File) -> String` renders uses/types/fns in AST order; functions emit the header plus one-space body lines (`doc`, `ex`, `let`/`do`, result). Expressions and types are prefix with single spaces, lists attach brackets (`[[1 2] [3]]`), floats keep stored text, strings use `ast::escape_str`; keywords/operators come from `lexicon` (only `:`, `[`, `]`, `=`, `"` are literal punctuation)
- `aleph fmt FILE` prints canonical text (exit 0); `-w` writes it in place; lex/parse diagnostics remain JSON with exit 1
- tests: canonical snippets covering every expression/type/item form, both SPEC §12 programs, every `tests/aph/*.aph` with `.out`/`.ast`, non-canonical inputs (spaces, trailing spaces, empty lines, item and body order), and a 500-case LCG AST generator (depth-limited, arity-respecting calls) checking `parse(print(ast)) == ast` and `same_ids`
- decisions: none (implements ADR-0003, ADR-0016, ADR-0017)
- next: T-009 F001 canonical check
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T12:30 T-009 done
- `aleph check`: after a clean parse, compares the source text with `printer::print_file`; any difference emits one F001 (`line`/`col` at the first differing line, col 1, msg `not in canonical form`, `canonical` = full canonical text) and exits 1; canonical input still reaches X001
- golden `tests/aph/fmt_*` cases: non-canonical `.aph` with `.fmt` expected `aleph fmt` output and `.diag` expected F001 JSON; harness runs `fmt` over every `.fmt` case and the existing check harness covers the `.diag` files
- tests: 128 passed (108 unit, 17 CLI, 3 golden)
- decisions: none
- next: T-010 type checker
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T12:45 T-008/T-009 review fixes done
- printer generator: `gen_type` emits `Named` (`point`); expressions include `Ctor` calls (`point` arity 2) and every builtin with its table arity (including 0-arity `args`); uses cover all capabilities with random paths containing `"`, `\`, newline and tab; docs, ints (`i64::MIN/MAX`, `0`) and floats (`0.5`, `-0.25`, `10.0`) are randomized
- `fmt -w` writes `FILE.aleph-tmp` in the same directory and renames it over `FILE`, so a failed write cannot truncate the source; test checks the temp file is gone
- lexer: raw control bytes inside string literals (`< 0x20` or `0x7F`) are L001; only `\n`/`\t` escapes are accepted (new lexer test)
- `print_file` documents the empty-module canonical text (`""`) and the parser-producible AST precondition
- F001 now reports the first differing byte column within the reported line (goldens and CLI test updated; `fmt_spaces` col 4, `fmt_body_order` col 2)
- `Use::canonical_text` is public and is the single `use`-line printer shared by the printer and the SPEC §3 sort key
- tests: 129 passed (109 unit, 17 CLI, 3 golden)
- decisions: none
- next: T-010 type checker
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T13:10 T-010 done
- `compiler/src/check.rs`: bidirectional type checker with `Type` (primitives, list/opt/res, records, fn, inference vars, `Error`), a substitution with occurs check, and `CheckOutput { diagnostics, records, signatures }`; `check(file, ast)` lowers type annotations (unknown `Named` -> T002 at the type node) and checks every function even after errors
- calls: user functions and record constructors use their lowered signatures; builtins instantiate `T/A/B` with fresh vars per call and per `@f`; `Primitive`/`Showable` bounds are deferred and flushed at the end of each function (T004), unresolved vars skipped; `@f` is accepted only when the expected type resolves to a function (T006 otherwise)
- rules: literals/vars, `+ - * / %`, `== !=` (T004 on non-primitives), comparisons, `and/or/not/neg`, `if`, lists, `empty`, field access (T003), `some`/`none`, `ok`/`err` (T010 without an expected `res`), `try` (T007 unless the enclosing function returns `res _ E`), `else`, `has`, `isok`; examples use an empty environment and check rhs against lhs; `main` must have no params and return `unit` or `res unit str` (T008)
- `compiler/src/main.rs`: `aleph check` runs lex/parse -> F001 -> type check; T diagnostics are printed with exit 1, a clean file still reaches X001
- diagnostics: new T002/T003/T004/T006/T007/T008 added to `diag::CODES` and `spec/diagnostics.md`; canonical type text (fn types `(A B) -> R`, unresolved vars `_`); ADR-0019 (Proposed)
- goldens: 20 `tests/aph/type_*.aph` + `.diag` covering T001 (ret/arg/binop/if/list/do/ctor/try), T002, T003 x2, T004 x4, T006, T007, T008 x2, T010; unit tests cover unify/occurs, no-cascade, fresh vars per call, sort/show/show-in-map bounds, fnrefs, try, else, main forms, and SPEC §12 + existing valid goldens clean
- tests: 143 passed (123 unit, 17 CLI, 3 golden)
- decisions: ADR-0019 (Proposed)
- next: T-012 capability checker C001/C002; T-014 examples runner (structural comparison, non-primitive examples allowed)
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T14:05 T-010 review fixes done
- `else` now propagates the expected type: the default branch gives `T` (checked against `expected` when present), then the value is checked against `opt T` (some/none/opt-typed) or `res T fresh` (ok/err/try/res-typed); `else err "x" 0` in an `i64` function is valid, and `else o 0` still uses the inferred opt/res payload
- operators probe operands before checking: a lone inference variable is unified with the concrete operand; two unresolved variables are unified with each other and their `numeric`/`primitive`/`i64` constraint is deferred (T004 after the function body) instead of reporting `_`; `ok + x 1` after `let x try err "x"` is valid
- `@f` to an effectful builtin (`print`, `eprint`, `read`, `write`) is T006, keeping capability checking on `Call` nodes only (`map @read xs` -> T006)
- `ok`/`err` with a concrete non-`res` expected report T001 (`expected` = that type, `found` = `res _ _`) and return `Error`; T010 is only for no expected type or an unresolved expected variable; `check_ok`/`check_err`/`check_try` return `Error` on failure
- recovery uses `contains_error` everywhere, so container types such as `list error` never cascade into T001; `unify` is transactional (substitution snapshot restored on failure)
- recursive records are allowed (`type node v:i64 next:opt node`; `node 1 none node` type-checks)
- `Type` now implements `Display` for the canonical diagnostics spelling (`list i64`, `res unit str`, `(i64 str) -> bool`, `_`, `error`), used by all diagnostics and available to T-016
- ADR-0019 updated with effectful FnRefs, recursive records and the Display spelling; new unit tests cover every review point
- tests: 150 passed (130 unit, 17 CLI, 3 golden)
- decisions: ADR-0019 (Proposed, updated)
- next: T-012 capability checker C001/C002
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T14:20 T-010 else/try correction done
- `else`: only syntactic `ok`/`err` are result-shaped (and `some`/`none` option-shaped); `try` and every other value are inferred, then required to be `opt T` or `res T E`. `else try r 0` with `r: res opt i64 str` is accepted (try yields `opt i64`), while `r: res i64 str` reports T001
- tests: 151 passed (131 unit, 17 CLI, 3 golden)
- decisions: ADR-0019 unchanged
- next: T-012 capability checker C001/C002
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T14:40 T-012 done
- `compiler/src/caps.rs`: `check_caps` walks every builtin Call (examples, statements, result) and reports C001 missing capability / C002 literal path not covered, sorted by (line, col); `covers` normalizes `/`, drops empty and `.` segments, rejects `..`, and distinguishes absolute paths; `capabilities` returns sorted, deduplicated (spelling, path) pairs
- C003 invalid capability for `..` prefixes and fs.read/fs.write without a path; added to diag::CODES and spec/diagnostics.md; ADR-0020 Proposed
- `aleph check` pipeline is now F001 -> T -> C -> X001; SPEC §12 second program passes C checks
- golden: `tests/aph/cap_c001_read|cap_c001_write|cap_c002_outside|cap_c002_dotdot|cap_c003_dotdot|cap_c003_nopath` with .diag, and `cap_valid` with the expected X001
- tests: 168 passed (148 unit, 17 CLI, 3 golden)
- decisions: ADR-0020 (Proposed)
- next: T-013 interpreter (reuse caps::covers for R003); T-014 examples runner
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T15:05 T-012 review fixes done
- builtins: tests pin effectful-without-capability to {eprint, print} and `path_arg < arity` for every builtin
- caps: missing `path_arg` in a call is now a `debug_assert!` (parsed calls cannot have wrong arity); empty `use` prefix is C003; C003 carries `found` (prefix or `""`) and distinct messages "invalid capability prefix" / "missing capability path" (code unchanged)
- ADR-0020 clarifies that `covers` is the lexical C002 filter and R003 must canonicalize the prefix and path (parent for not-yet-existing write targets) before reusing the comparison; POSIX only, leading `//` is `/`
- goldens: `cap_c003_dotdot`/`cap_c003_nopath` updated, `cap_c003_empty` added
- tests: 171 passed (151 unit, 17 CLI, 3 golden)
- decisions: ADR-0020 (Proposed, updated)
- next: T-013 interpreter (R003 canonicalizes before `caps::covers`); T-014 examples runner
- agent: opencode/deepseek-v4.1-flash
