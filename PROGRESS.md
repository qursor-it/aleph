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

## 2026-11-15T15:20 T-013 done
- `compiler/src/interp.rs`: values, structural equality, all SPEC §5 expressions (strict/left-to-right, short-circuit `and`/`or`, branch-only `if`, `try` early return through nested expressions) and all §5.1 builtins; checked i64 arithmetic (R001/R002), canonical-path capability checks with symlink resolution (R003), 10 000-call depth limit (R004), `show`/`int`/`float` semantics per ADR-0021
- `run_main` streams through locked `BufWriter` stdout/stderr on a 256 MiB thread; `run_main_with` keeps explicit writers for tests
- `aleph run FILE [ARGS..]` wired: X002 -> lex/parse -> T -> C (no F001), X003 when `main` is missing, trap JSON on stderr with exit 70, `err e` on stderr with exit 1
- goldens: `tests/aph/run_*.aph` with `.out`, `.err` and `.exit` for SPEC §12 fib, records, map/filter/fold, strings, show, opt/res/try, main-err and R001/R002 traps; `compiler/tests/run_golden.rs` also verifies each run golden is canonical and type/cap-clean
- diagnostics: X003 added to `diag::CODES`, `spec/diagnostics.md` and ADR-0015; R004 added in part 1
- tests: 206 passed (179 unit, 22 CLI, 3 lexer/diag goldens, 2 run goldens)
- decisions: ADR-0021 (Proposed); ADR-0015 updated with X003
- next: T-014 examples runner (`aleph test`, examples inside `aleph check`)
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T15:45 T-013 review fixes done
- R003: replaced the parent-only fallback with `resolve_lenient` (rejects `..`, canonicalizes the longest existing ancestor and appends the missing suffix) for both prefix and target; existing symlinks must canonicalize (dangling → R003); compare with `Path::starts_with`. Added tests for dangling-symlink writes (nothing created outside), nested missing reads (`err "not found"`), directory reads, and `use fs.write "out.txt"` on first write
- R005: `range` traps "allocation limit exceeded" when `b - a > 10_000_000` or the subtraction overflows; added to `diag::CODES` and `spec/diagnostics.md`
- R004: added an evaluation-depth budget alongside the 10 000-call limit. Measured 64 000 unsafe (native overflow) and 12 000 too low for the 5 000-recursion test; 20 000 passes both that test and a 250-deep nested `+` recursion stress test on the 256 MiB thread
- io errors: explicit `is a directory`, `not a directory`, `directory not empty`, `read-only filesystem`; fallback is now `io error`
- stderr ordering: stdout is flushed before every stderr write and stderr immediately after; covered by a shared-sink unit test
- ADR-0021 documents the resolver, R005, the measured depth budget, flush ordering, io texts, and TOCTOU/hard-link v0.1 limitations (no libc/openat2)
- tests: 213 passed (186 unit, 22 CLI, 3 lexer/diag goldens, 2 run goldens)
- decisions: ADR-0021 (Proposed, updated)
- next: T-014 examples runner (`aleph test`, examples inside `aleph check`)
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T16:20 T-014 done
- `compiler/src/examples.rs`: `aleph check` reports E001 at the fn header for pure non-`main` functions without examples and E002 at the example node for mismatches/traps; effectfulness is a deterministic fixpoint over direct calls, user-fn calls and `@fnref` arguments (`print`, `eprint`, `read`, `write`); all examples run even when some fail, diagnostics sorted by `(line, col)`
- `interp::literal` renders values in canonical Aleph literal syntax (quoted strings, prefix constructors, records as `NAME f1 f2`); E002 `expected`/`found` use it, traps become `found = "trap R00x"` and `expected = "?"` when the rhs did not evaluate; `try` at example top level yields its `err e`
- `aleph test FILE` wired: L/P -> T -> C -> examples only (no F001, no E001), JSON lines on failure, silent exit 0 when clean; `aleph check` pipeline is now read -> L/P -> F001 -> T -> C -> E001+E002 and no longer returns X001
- goldens: `ex_e001_missing`, `ex_e002_wrong`, `ex_e002_trap`, effectful-without-ex clean, SPEC §12 fib/words clean; `cap_valid.diag` is now empty; `.diag` harness treats an empty file as exit 0/empty stdout; run_* goldens gained missing pure-function examples and the run harness now requires `aleph check` to be silent
- decisions: ADR-0022 (Proposed)
- tests: 226 passed (197 unit, 24 CLI, 3 lexer/diag goldens, 2 run goldens)
- next: T-015 golden harness expansion (SPEC §12 + 15 more programs)
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T16:50 T-014 review fixes done
- examples are hermetic: `Interp::with_deny_fs` makes every `read`/`write` trap R003 before path resolution; examples always enable it, so an fs example yields `E002 found "trap R003"` and never writes a file
- effectfulness now walks only fn statements and result, never examples; a pure body with an effectful example no longer exempts an unexampled pure caller from E001
- trap reporting: lhs-only trap -> `expected` = rhs value, `found` = lhs trap; rhs-only trap -> `expected` = rhs trap, `found` = lhs value; both -> lhs trap found, rhs trap expected
- ADR-0022 updated with hermetic evaluation, body-only effectfulness, trap direction and "value notation" (not guaranteed re-parseable: `none`, `[]`, non-finite floats)
- goldens/tests: `ex_fs_denied` (E002 R003) plus a CLI test asserting `tests/aph/out-hermetic.txt` is not created; unit tests for example-independent effectfulness, rhs-only trap and both-trap cases
- tests: 231 passed (201 unit, 25 CLI, 3 lexer/diag goldens, 2 run goldens)
- decisions: ADR-0022 (Proposed, updated)
- next: T-015 golden harness expansion (SPEC §12 + 15 more programs)
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T17:20 T-015 done
- merged `compiler/tests/lexer_diag.rs` and `compiler/tests/run_golden.rs` into `compiler/tests/golden.rs`; kept every check (diag stdout/exit, fmt/ast goldens, run stdout/stderr/exit, canonical + silent check) and collect all failures before one assert
- new orphan rule: every `tests/aph/NAME.aph` must have at least one of `.out`, `.diag`, `.ast`, `.fmt`
- run goldens execute in a unique `std::env::temp_dir()` directory containing a copy of the source and `tests/aph/data/`, with cwd set there; `.args` is one arg per line, optional `.exit`/`.err`, and `.files` entries are compared with `tests/aph/expect/<relpath>` (parent directories are created before the run)
- added the 15 `bench/tasks.md` programs as `bench_*.aph` with goldens: fib, total, evens, primes, fizzbuzz, palindrome, wordcount, linecount, maxmin (+ `bench_maxmin_none` with stderr/exit 1), points, reverse, gcd, upper, histogram, collatz; added `tests/aph/data/in.txt` (5 words, 2 non-empty lines) and `expect/out/up.txt`
- trap goldens now name the source by its basename because run goldens execute from the temporary directory (`file` field is `run_trap_*.aph`)
- SPEC §12 examples remain covered by `ex_spec12_*` and `run_spec12_fib`
- language friction: no string repetition/char access (`split s ""` + `join` + `map`), no `sum` over `f64`/`sqrt` (Newton loop in `bench_points`), `fold` requires accumulator type equal to the function result (res accumulators need a `res` parameter), and `mkdir` is absent so the harness creates `.files` parent directories
- tests: 232 passed (201 unit, 25 CLI, 6 golden)
- decisions: none
- next: T-016 `aleph explain`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T18:10 T-015 review fixes done
- golden harness hardening: `.files` entries are validated as plain relative paths (absolute, `.`/`..`, root and prefix components are test failures) before any temp/expect path join; temp dirs are RAII (`Drop` removes them) and created with `fs::create_dir`, retrying on EEXIST; `read_lines` treats only `NotFound` as empty and reports other IO errors; sidecar consistency now catches sidecars without `.aph` and `.args`/`.exit`/`.err`/`.files` without `.out`; spawned `aleph` processes are polled for at most 30 s, then killed and reported as failures
- documented harness path conventions and the `bench_maxmin`/`bench_maxmin_none` twin in `compiler/tests/golden.rs`
- recorded benchmark formats: collatz prints the sequence length as the number of terms including 1 (`collatz 1 = 1`, `collatz 3 = 8`); gcd prints `gcd A lcm B`; histogram prints one `N <stars>` line per argument (e.g. `2 **`); maxmin prints `max M min N` and the no-args twin writes `no numbers` to stderr and exits 1; upper writes `out/up.txt` and prints nothing; points prints a decimal f64 perimeter
- tests: 232 passed (201 unit, 25 CLI, 6 golden)
- decisions: none
- next: T-016 `aleph explain`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T15:17 T-016 done
- `compiler/src/explain.rs`: typed `Report` (`file`, `capabilities`, `types`, `functions`, `entry`) built after read -> lex/parse -> type check -> caps; capabilities always first via `caps::capabilities`, types sorted by name, functions in canonical file order, entry `main`/null
- per function: canonical header and params from `printer::print_type`, doc, `effectful` from `examples::effectful_fns`, examples from `printer::print_expr`, sorted unique user calls and builtins from the body incl. `@fnref` (examples excluded, documented)
- text format documented in the module: `capabilities: CAP ["path"]` / `none`, `type NAME f:T ...`, `fn ...`, indented `doc`/`effectful`/`examples`/`calls`/`builtins`, `entry: main|-`; all lists are `Vec`/`BTreeSet`, never hash-ordered
- `--json`: serde structs in declaration order, two-space pretty print plus trailing LF; `aleph explain` wired in `main.rs` with the diagnostics path unchanged (JSON lines, exit 1)
- golden harness: `.explain` runs `aleph explain`, `.explain.json` runs `aleph explain --json`; both accepted as companions and `.explain.json` is checked as a two-part sidecar; added both formats for `ex_spec12_fib`, `ex_spec12_words` and the new `explain_records` program
- CLI test: ill-typed file prints `T001`, exits 1, byte-identical over two runs; valid record program is also byte-identical
- tests: 242 passed (208 unit, 26 CLI, 8 golden)
- decisions: none
- next: T-017 Tlön runtime in C
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T15:29 T-016 review fixes done
- explain text no longer hard-codes keywords: function headers and record lines use `lexicon::keyword`/`lexicon::operator`; the JSON schema field names stay as specified
- `doc` is now quoted and escaped (`doc: "..."`) or `doc: -` when absent, keeping null distinct from `""`; explain text goldens regenerated
- shared pre-order walker `ast::walk_expr` added (with a unit test) and used by `explain`, `caps` and `examples`; the three local copies are gone
- added empty `tests/aph/explain_records.diag` (check is silent)
- ADR-0023 (Proposed) records the text layout, JSON field names/order, body-only call graph and determinism rules as a contract
- tests: 243 passed (209 unit, 26 CLI, 8 golden)
- decisions: ADR-0023 (Proposed)
- next: T-017 Tlön runtime in C
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T19:10 M1 re-review fixes G1/G2
- `fmt -w` now writes a unique sibling temp (`FILE.aleph-tmp-<pid>-<n>`, `create_new` with AlreadyExists retry), copies the target's permissions, writes + `sync_all`, then renames over the target; every post-create failure removes the temp. A symlinked source is resolved with `canonicalize` so the link survives and its target is updated
- `ast`: `ExprKind::If` fields serialize as `then`/`else`; new canonical `tests/aph/ast_if.aph` + `.ast` golden
- lexer: raw control characters in strings use `char::is_control()` (C0, DEL and C1) and report L001; U+0085 added to the test
- lexicon test asserts BINARY_OPERATORS ∪ PUNCTUATION == OPERATORS as spelling sets in both directions
- parser: `count_extra_expressions` stops at the first unparsable leftover and returns P001 at that token, so `+ 1 2 3 ]` no longer inflates P002
- ADR-0018 records that files with T005 are intentionally not formatted by `aleph fmt`
- tests: 247 passed (210 unit, 29 CLI, 8 golden)
- decisions: ADR-0018 (Proposed, updated); ADR-0023 unchanged
- next: T-017 Tlön runtime in C
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T18:05 T-017 done
- `tlon/tlon.h` + `tlon/tlon.c`: tagged `AlValue` ABI (`AlStr`/`AlList`/`AlRecord`/`AlBox`/`AlFn`), single 16-byte-aligned bump arena, location stack plus eval/call depth R004 mirror, JSON traps exit 70, checked i64 R001/R002, every non-higher-order SPEC §5.1 builtin, `resolve_lenient` R003 and `aleph_main_run` (pthread 256 MiB, unit/ok/err handling)
- `show`/`literal` match the interpreter; f64 uses shortest round-trip with Rust's half-away-from-zero tie rounding (fixes exact `.625` cases) and the `-4`/`16` exponent thresholds
- `compiler/tests/tlon.rs` compiles `tlon/tlon.c` + `tlon/test_tlon.c` with `nice -n 15 cc -std=c99 -O1 -Wall -Wextra -Werror -pthread`, runs trap/builtin/format checks, and compares ~2000 deterministic f64 bit patterns with Rust `{:?}`; skips only when `cc` is missing
- decisions: ADR-0024 (Proposed)
- tests: 249 passed (210 unit, 29 CLI, 8 golden, 2 tlon)
- next: T-018 C codegen for expressions/functions and `aleph build`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T18:27 T-017 review fixes done
- `tlon/tlon.c`: fixed the last `split` segment (runs to `source_len` when no separator remains); embedded-NUL `read`/`write` paths now return `err "invalid input"`; arena chunk headers and every `len + 1` allocation are overflow-checked to R005; arena chunk bases are explicitly rounded to 16 bytes; `write()` returning 0 is `err "io error"` and `close()` errors are ignored like Rust; `aleph_main_run` exits 70 (one-line stderr) instead of falling back to the default stack when pthread setup fails; function equality now compares `AlFnKind` plus name like `FnTarget`; `resolve_lenient` starts its ancestor scan at `count - 1` like Rust and preserves trailing-slash behaviour
- `tlon/tlon.h`: documented that `AlLoc` pointers passed to `aleph_eval_enter`/`aleph_call_enter` need static storage; added `AlFnKind` and `aleph_fn_kind`
- `tlon/test_tlon.c`: added split driver mode, function-kind equality checks, embedded-NUL path checks, trailing-slash path checks
- `compiler/tests/tlon.rs`: new `--split` parity test against Rust `str::split`; `ALEPH_REQUIRE_CC=1` turns a missing `cc` from a skip into a failure
- tests: 250 passed (210 unit, 29 CLI, 8 golden, 3 tlon)
- next: T-018 C codegen for expressions/functions and `aleph build`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T18:45 T-017 cycle-2 review fixes done
- `tlon/tlon.c`: `read`/`write` now perform the capability check on the full length-aware path first (NUL paths outside a prefix trap R003; covered NUL paths return `err "invalid input"`); `aleph_str_n(NULL,len)` zero-fills and `aleph_record(...,NULL,n)` zero-initializes fields; `pthread_join` failure exits 70 with a one-line stderr message; GCC/Clang requirement documented in the header/ADR
- `tlon/tlon.h`: documented static/program-duration lifetime for `aleph_set_file`/`aleph_set_uses`/`aleph_set_args` pointers and the `__builtin_*_overflow` compiler requirement
- `tlon/test_tlon.c`: added NUL-path-outside-prefix R003, existing-symlink-escape R003 for read and write, NULL-source zeroing checks
- `compiler/tests/tlon.rs`: unchanged; full suite still green
- tests: 250 passed (210 unit, 29 CLI, 8 golden, 3 tlon)
- next: T-018 C codegen for expressions/functions and `aleph build`
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T21:30 T-018 partial
- `check`: `CheckOutput::expr_types` records the resolved type of every expression node (used for record field indexes in codegen)
- `cgen`: deterministic C99 emitter for literals, variables, calls, constructors, lists, `if`, `and`/`or`, `try` (epilogue goto with depth restore), records/field access, opt/res helpers and all non-higher-order builtins; per-function temps and arrays declared at the top; static `AlRecordInfo`/`AlLoc` tables and forward declarations; R004 depth mirrors `eval`/`call_user`
- `build`: read -> parse -> type/capability check -> cgen -> embedded `tlon.c`/`tlon.h` in a fresh temp dir -> `$CC -std=c99 -O2 -pthread`; X004 on cc failure (stderr forwarded), X005 for unsupported constructs, usage exit 2 when `-o` is missing; generated `main` sets file/uses/args and calls `aleph_main_run`
- runtime: `aleph_f64_bits` for exact literal bit patterns; `AlFn.name`/`AlRecordInfo.name` documented as requiring static storage
- diagnostics: X004/X005 added to `diag.rs`, `spec/diagnostics.md` and ADR-0015
- tests: cgen determinism/literal/unsupported unit tests; CLI build+run tests for SPEC §12 fib (6765), records/opt/res/try, R001 trap parity, main `err`, args, missing `-o` and X005 for higher-order builtins; `tlon.rs` f64 tie case `137839762462415.625` (+ negative); `scripts/check.sh` exports `ALEPH_REQUIRE_CC=1`
- decisions: ADR-0025 (Proposed)
- tests: 260 passed (213 unit, 36 CLI, 8 golden, 3 tlon)
- next: T-019 must implement `map`/`filter`/`fold` through `AlFn` function pointers (and function values), then T-018 can be marked done and differential tests (T-020) can cover the remaining benchmark programs.
- agent: opencode/deepseek-v4.1-flash


## 2026-11-15T22:20 T-018/T-019 done
- `cgen`: every `@f` (user function, record constructor, pure builtin) now gets one `static AlValue aleph_ref_<kind>_<name>(AlValue *args, size_t n)` wrapper and one `static const AlFn aleph_fnref_<kind>_<name>` value; `FnRef` emits `aleph_fn_static(&...)`
- user-function wrappers do `aleph_call_enter(NULL)`/`aleph_call_leave()` around the generated call, so R004 depth and trap locations match `interp::call_target` (current location is the enclosing higher-order call); constructor/builtin wrappers add no call frame, also matching the interpreter
- `tlon`: `aleph_fn_static` plus `aleph_b_map`, `aleph_b_filter` and `aleph_b_fold` loop left to right and call `fn->code`; `aleph_call_enter(NULL)` documented as "use current location"
- X005 is no longer emitted anywhere; it stays in `diag.rs`, `spec/diagnostics.md` and ADR-0015 as a reserved code
- tests: build+run parity for map/filter/fold with user functions, constructor/builtin refs (`@box`, `@show`, `@len`), fold with a `res` accumulator, and an R002 trap inside a mapped function; every `tests/aph/bench_*.aph` builds natively and matches its `.out` (with `.args`, `data/` and cwd like the golden harness)
- decisions: ADR-0025 updated with the function-value/wrapper design and reserved X005
- tests: 264 passed (213 unit, 40 CLI, 8 golden, 3 tlon)
- next: T-020 differential tests can now cover all golden/benchmark programs; T-021 benchmark harness is unblocked.
- agent: opencode/deepseek-v4.1-flash


## 2026-11-15T23:10 T-018/T-019 review fixes done
- generated names: function return slot is `aleph_ret_`, entry-wrapper locals are `aleph_args`/`aleph_count`/`aleph_value`, wrapper parameters are `aleph_args`/`aleph_n`; every non-user generated name now has an underscore pattern Aleph identifiers cannot form, so `let result`, params named `result` and similar bindings cannot collide
- list literals and record-constructor field arrays are built with `aleph_list_new` + `u.list->items[i]` on the arena; no generated C stack arrays remain
- `tlon`: `main_thread` records the stack base and `aleph_call_enter` traps `R004` when less than 1 MiB of the nominal 256 MiB remains; guard disabled outside the main thread; ADR-0025 documents that native may trap R004 earlier only for pathological frames
- docs: corrected `aleph_call_enter` location-stack wording in ADR-0025/ADR-0024/tlon.h (only `aleph_eval_enter` pushes the location; `call_enter` uses `loc` for a trap, NULL = current)
- `check`: `expr_types` is resolved once more with the final substitution after all functions are checked; cgen rejects a top-level `Var`/`Error` with `X006` "internal compiler error" instead of guessing (nested inference variables, e.g. an unconstrained `res` error type, are allowed)
- build temporary directories are created with mode 0700 on Unix
- tests: c_string escaping (quote, backslash, newline, tab, `??=`, UTF-8 é, NUL), reserved-name builds, X004 with `CC=/bin/false`, X003 without `main`, and a 3000-element list literal recursing 5000 deep with no SIGSEGV; `CC` probe in CLI/Tlön tests uses `$CC` when set (bare executable path)
- tests: 271 passed (216 unit, 44 CLI, 8 golden, 3 tlon)
- next: T-020 differential tests over all golden/benchmark programs; T-021 benchmark harness.
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T23:40 T-020 done
- new `compiler/tests/differential.rs`: every `tests/aph/NAME.aph` with `NAME.out` runs the interpreter (`aleph run`) and the C backend (`aleph build` + `./prog`) in independent hermetic temp dirs (source, `data/`, `.args`, 30 s timeout, RAII cleanup); it compares stdout/stderr/exit byte for byte between engines and against `.out`/`.err`/`.exit`, and compares every `.files` output across both runs and `tests/aph/expect/`; builds are sequential and all failures are collected into one assertion
- new run goldens: `run_show_floats` (0.1, 1e20, 1e-7, -0.0, 0.0001, inf/-inf/NaN via division), `run_strings_unicode` (`split ""` on héllo, NBSP/EM-SPACE trim, byte count, empty split), `run_sort_f64` (NaN, ±inf, ±0), `run_parse_edge` (i64 min/max/overflow, +1, -0, invalid ints/floats, -0.0), `run_trap_map` (R002 in a map callback, node n9), `run_trap_cap` (R003 on a computed path outside the prefix, node n12), `run_eprint` (stdout/stderr interleaving), `run_deep_recursion` (4000 successful calls)
- divergences found between interpreter and native for these probes: none; `tlon`/`cgen` semantics already matched
- removed the redundant stdout-only `build_all_bench_programs_match_expected_stdout` CLI test (differential.rs now covers every `.out` case, including all benchmarks)
- review MINORs: ADR-0021 notes native may trap R004 earlier via the stack guard; ADR-0024 documents the IEC 60559 f64-division assumption and forbids `-ffast-math`; ADR-0025 records arena-list record fields and the `aleph_ret_` slot; generated `main` guards `argc == 0`; `aleph_main_run` clears the stack-guard state after join; `test_tlon.c` directly tests `aleph_b_map`/`filter`/`fold` and `aleph_fn_static`
- tests: 271 passed (216 unit, 43 CLI, 1 differential, 8 golden, 3 tlon)
- next: T-021 benchmark harness `bench/run.sh`.
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T23:55 T-020 review fixes done
- extracted the integration-test harness into `compiler/tests/common/mod.rs` (repo root, hermetic temp dirs, source/data/args setup, sidecar readers), shared by `golden.rs` and `differential.rs`; the process runner now drains stdout/stderr on reader threads while polling `try_wait`, avoiding pipe-buffer deadlocks; `run_big_output` prints 262 144 bytes and passes in both harnesses
- merged-stream ordering: `common::run_merged` opens one file and passes `try_clone()`s to both `Stdio` handles; differential runs cases with `.merged` a second time and compares the combined stream between engines and against the golden; added `run_eprint.merged`, `run_trap_overflow.merged` (print then trap) and `run_main_err.merged`, plus interpreter-only merged checks in `golden.rs`
- extended `run_show_floats` with 137839762462415.625, 123456789012345680.0, 1e16, 1e15, 1e-5, 1e-7 and -1e-7 (all match natively); `run_trap_cap` now uses a relative computed path (`cat "da" "ta2/x"`) and new `run_trap_cap_dotdot` exercises a runtime `..` path (`cat "data/" "../x"`) -> R003 without touching `/etc`
- `cgen`: generated `main` no longer emits the redundant `aleph_set_args` (`aleph_main_run` records the arguments); ADR-0025 shows the guarded `argc > 0 ? ... : NULL` form
- tests: 271 passed (216 unit, 43 CLI, 1 differential, 8 golden, 3 tlon)
- next: T-021 benchmark harness `bench/run.sh`.
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T21:13 T-021 done
- `bench/ref/py/NAME.py` and `bench/ref/c/NAME.c`: Python 3 (stdlib) and C99 (libc) references for all 15 `bench/tasks.md` programs; they consume the same `.args`, `data/in.txt` and write `out/up.txt`, matching the Aleph golden stdout/stderr/exit byte for byte
- `bench/run.sh`: builds `target/release/aleph` once with `CARGO_BUILD_JOBS=1` + nice (or uses `ALEPH`), then per task in a temp dir with `data/` and `out/` compiles the C ref (`cc -O2`) and native Aleph (`aleph build`), verifies native/interpreter/Python/C against `.out`/`.err`/`.exit`/`.files` before timing, runs each `BENCH_RUNS` times (default 3) under `/usr/bin/time -f "%e %M"` (bash `time` + rss `n/a` fallback), and prints a markdown table; supports `BENCH_TASKS`; any mismatch exits non-zero
- `bench/README.md` and `bench/results-sample.md`: usage, measured quantities, `/usr/bin/time` 10 ms resolution and shared-VPS caveats; `BENCH_SCALE` is documented as unsupported because task sizes are fixed by the goldens
- `compiler/tests/bench_refs.rs`: two fast tests compile/run each C ref and each Python ref once in hermetic dirs and compare stdout/stderr/exit/written files with the Aleph goldens; skips without `cc` unless `ALEPH_REQUIRE_CC=1`, skips Python with a notice when `python3` is absent
- tests: 273 passed (216 unit, 2 bench_refs, 43 CLI, 1 differential, 8 golden, 3 tlon)
- decisions: none (harness only)
- next: M3 tasks (T-022 llms.txt first).
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T21:25 T-021 review fixes done
- timing: `bench/run.sh` now exports `LC_ALL=C` and measures each run with bash `EPOCHREALTIME` around the process itself (microseconds), reporting the median in ms with two decimals; `/usr/bin/time -f "%M"` is used only for peak RSS, and a missing `/usr/bin/time` only makes RSS `n/a`
- memory: the table is now `task | native ms | interp ms | python ms | c ms | native KB | interp KB | python KB | c KB` (per-engine peak RSS)
- scaling: `BENCH_SCALE=1` appends a bench-only `fib35` row built from `bench/heavy/fib35.aph` + `.out` and `bench/ref/{c,py}/fib35.*`; the default 15 tasks stay short and startup-dominated
- `points` refs now use the same 40-iteration Newton sqrt as the Aleph source (equal work, output still `12.0`); `reverse.c` grows its token array dynamically instead of a fixed 256 bound
- `bench_refs.rs`: `CC` set but not runnable now panics loudly; `fib35` is covered too; `cc` missing without `CC`/`ALEPH_REQUIRE_CC` still skips
- `time_engine`/`rss_engine` take the expected exit code, removing the dead nonzero-exit timing path; README explains timing, per-engine RSS, `BENCH_SCALE`, fixture-range assumptions (no R001 emulation) and removes the incorrect `-O2` constant-folding claim
- tests: 273 passed (216 unit, 2 bench_refs, 43 CLI, 1 differential, 8 golden, 3 tlon); `BENCH_RUNS=3 bench/run.sh` green and sample table updated
- decisions: none (harness only)
- next: M3 tasks (T-022 llms.txt first).
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T21:35 T-022 done
- new `compiler/src/llms.rs`: deterministic `render()` built from `include_str!` sources (SPEC purpose paragraph and headings by number, `spec/diagnostics.md` code list, ten verbatim `tests/aph` examples), generated builtin table from `builtins::BUILTINS` (name/signature/effect/capability) and keywords from `lexicon`; estimated tokens = `ceil(chars/4)`
- `aleph llms` (no FILE) prints `render()`; `cli::requires_file` marks file-less commands and keeps existing commands/tests unchanged
- root `llms.txt` checked in: 6792 chars, ~1698 estimated tokens (<= 1750); exactly 10 fenced examples: run_show, run_records, bench_fib, bench_fizzbuzz, run_higher_order, cap_valid, ex_spec12_words, run_eprint, bench_total, run_opt_res
- `compiler/tests/llms.rs`: root byte-equal to render (with regenerate hint), deterministic, CLI no-file, token bound, exactly 10 fenced examples verbatim, and every example parses/canonical/type/capability/example-clean via library calls
- decisions: ADR-0026 proposed (generation, sources, estimate, regenerate with `cargo run --quiet --bin aleph -- llms > llms.txt`)
- tests: 282 passed (219 unit, 2 bench_refs, 43 CLI, 1 differential, 8 golden, 6 llms, 3 tlon)
- next: T-023 `fix` suggestions for the most common diagnostics.
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T21:51 T-022 review fixes done
- drift detection: `llms.rs` pins hand-written FNV-1a 64 hashes for `spec/SPEC.md`, `spec/grammar.ebnf`, ADR-0018/0020/0021/0022; `compiler/tests/llms.rs` recomputes them and fails with `source X changed: review llms.rs digest, then update hash`; ADR-0026 documents the review step
- builtin bounds: `sort` now renders `list T->list T (T primitive)`, `show` renders `T->str (T primitive or list of primitive)`; unit test covers every `BuiltinSpec::bounds` entry
- added compact `## Semantics` block: T005 name reuse, hermetic/global-only examples, print/eprint LF, `show` spellings, `at`/`slice`/`range`/`sort`/`count`/`split`/`int`/`float`, `main` err -> stderr + exit 1
- capabilities text now covers relative/absolute POSIX prefixes, C003 for empty/`..`/missing fs path, and `..` never covered
- keywords come from `lexicon::keyword_names()` with an exact-equality test; canonical order rules spell out use/type/fn sorting, `main` last, doc/ex/stmt/result order
- examples rebalanced (10): `llms_ops` (new golden, 426 bytes), run_strings, bench_upper, run_opt_res, run_higher_order, run_records, bench_fib, run_eprint, run_args, run_show; covers sort, at/else, slice, push, rev, sum, cat, count, upper, contains, float, fs.write, and/or/not/neg, ==/!=
- `aleph llms` now rejects FILE and flags with usage exit 2; builtin capability column has exhaustive Capability arms (no `?`)
- root `llms.txt` regenerated: 7674 chars, ~1919 estimated tokens (test bound raised to 1950, still < 2000 target)
- tests: 286 passed (221 unit, 2 bench_refs, 43 CLI, 1 differential, 8 golden, 8 llms, 3 tlon)
- next: T-023 `fix` suggestions for the most common diagnostics.
- agent: opencode/deepseek-v4.1-flash

## 2026-11-15T22:05 T-022 cycle-2 review fixes done
- capabilities: `use net.host "x"`, `use env`, `use time` parse in v0.1 but have no builtins until v0.2
- semantics: added `range` > 10,000,000 elements -> R005 and > 10,000 calls or deep evaluation -> R004; `upper`/`lower` ASCII-only; `ok`/`err` require an expected `res` type else T010
- canonical order now says `use` sorted by full canonical line text
- drift detection: added `spec/diagnostics.md` to `SOURCE_DIGESTS` (FNV-1a 64 `0x4af2f95d694ee464`); test asserts the exact seven-path digest set as well as each hash
- root `llms.txt` regenerated: 7757 chars, ~1940 estimated tokens (<= 1950)
- tests: 286 passed (221 unit, 2 bench_refs, 43 CLI, 1 differential, 8 golden, 8 llms, 3 tlon)
- next: T-023 `fix` suggestions for the most common diagnostics.
- agent: opencode/deepseek-v4.1-flash

## 2026-11-16T01:10 T-032 done
- `sort` on f64 put a NaN with the sign bit set before `-inf`: `0.0 / 0.0` is a negative NaN on x86-64 and a positive one on arm64, so `run_sort_f64` failed on x86-64 in both engines
- `interp::compare_values` and `tlon.c` `f64_total_cmp` now order every NaN after `inf`, whatever its sign or payload; ADR-0024 text updated
- `tlon/test_tlon.c` expectation changed from `[NaN -inf ... inf NaN]` to `[-inf ... inf NaN NaN]`: the old one encoded the platform-dependent order
- agent: manual (Bernardo Manfriani)

## 2026-11-16T01:10 T-033 done
- `cgen.rs`: every function emitted as `aleph_slow_<name>` (old instrumented body, `AL_COLD`) and `aleph_fn_<name>` (fast body: `int64_t`/`double`/`bool` temps, params and results; no `aleph_eval_enter/leave`); fast body falls back to slow when `eval_depth + max_eval_depth(fn) >= 20000`
- `tlon.h`: inline fast helpers (`aleph_fast_ok`, `aleph_fast_call_enter/leave`, `aleph_loc_push/pop/here`, `aleph_fast_add..neg`, `aleph_box_*`, `aleph_f64_from_bits`); `tlon.c` exports depth/location state as `aleph_g_*`
- new goldens at the R004 limit: `run_trap_depth_eval`, `run_trap_depth_map`, `run_depth_near_limit` (4998 ok, 4999 traps); interpreter and native byte-identical
- fib35 on x86-64: 1.71 s -> 0.115 s (CPython 1.12 s, C 0.02 s)
- tests: 288 passed, clippy clean
- decisions: ADR-0027 (Proposed)
- next: T-023; optional later: drop the location slot/stack guard cost per call if profiling shows it matters
- agent: manual (Bernardo Manfriani)

## 2026-11-15T22:30 T-023 done
- certain `fix` suggestions per ADR-0028: `check` attaches `replace` for F001 (node n0, canonical file text) and for T001 `ok`/`err`/`some`/`has`/`isok`/`try`/`show` wrappers only when expected/found are fully resolved; `caps` attaches `insert` at n0 for C001/C002 when the read/write path is a non-empty relative literal without `..` (escaped with `ast::escape_str`)
- no fixes for ambiguous result sides, unresolved/error types, computed/empty/`..` paths, or any other diagnostic; P002/T007 untouched
- new ADR-0028 Proposed: the ten rules, certainty conditions, and the meaning of insert at n0 (canonical order applies)
- new `tests/aph/fix_*.aph` + `.diag` (F001, T001 rules 2-8, C001 read/write, C002); existing F001/C001/C002 goldens updated with their fixes; `compiler/tests/fixes.rs` applies every golden fix, canonicalizes, rechecks that the failing code is gone
- unit tests per rule plus negative cases in `check.rs`/`caps.rs`; `printer::print_expr` is public for the apply test
- `llms.rs`: examples now read `ex E = E`+ (unless `main`/effectful), top-level fn/type names are unique and not builtin/type names (P001), `fix` may be replace/insert; `llms.txt` regenerated at 7797 chars (~1950 tokens); no digested source changed
- tests: 295 passed (229 unit, 2 bench_refs, 43 CLI, 1 differential, 1 fixes, 8 golden, 8 llms, 3 tlon)
- decisions: ADR-0028 proposed
- next: T-024 `aleph edit`.
- agent: opencode/deepseek-v4.1-flash (high); review + absolute-path guard by orchestrator (claude-opus-5.5), not an independent review

## 2026-11-16T16:30 T-024 done
- `aleph edit FILE --json EDITS [-w]` (new `edit.rs`, ADR-0029): EDITS is a JSON array of `{op,node,text?,position?}`; ids refer to the input AST; all edits are applied together, then the text is reparsed and printed canonically
- replace: n0, use/type/fn items, ex/let/do lines, expressions; delete: items and body lines; insert: items at n0, body lines at a fn (`start`/`end`) or next to a line (`before`/`after`)
- errors X007 invalid edits json, X008 unknown node id, X009 edit not allowed for node kind, X010 edit does not parse, X011 conflicting edits (pre-order id ranges); added to `spec/diagnostics.md` and `diag::CODES`
- stdout gets the edited text; `-w` writes atomically (same path as `fmt -w`) and prints `aleph check` diagnostics of the new file
- `ast::walk_expr_mut`, `printer::print_fn/print_typedef` and `ast::collect_ids` made crate-visible for edit
- `llms.rs`: diagnostic codes written as ranges (`T001-T008 T010 ...`) to stay under the 1950-token budget; diagnostics.md digest updated; `llms.txt` regenerated (7722 chars)
- tests: 307 passed (edit unit tests, cli edit print/-w/reject, cli parse test, collapse_codes test)
- decisions: ADR-0029 proposed
- next: T-025 MCP server (ADR-0030), T-026 tokens (ADR-0031)
- agent: orchestrator (claude-opus-5.5) directly, OpenCode quota exhausted; not independently reviewed

## 2026-11-16T17:30 T-025 done
- `aleph mcp` (new `mcp.rs`, ADR-0030): hand-written JSON-RPC 2.0, newline-delimited stdio, `serde_json` only (MCP SDK crate scored 3/10 vs 9/10)
- methods initialize, ping, shutdown, tools/list, tools/call, resources/list, resources/read; notifications ignored; standard error codes
- tools spec (compact/full), check, test, fmt, ast, explain, edit, run; source tools write inline `source` to a fresh temp dir and run the same binary there with empty env, null stdin, timeout (default 5 s, max 60 s), 1 MiB cap per stream; result JSON `{exit_code,timed_out,truncated,stdout,stderr}`
- `run` rejects absolute `use` prefixes; `build` omitted (needs cc, binary output)
- resources aleph://llms.txt and aleph://spec/{SPEC.md,grammar.ebnf,diagnostics.md}; `llms::SPEC/GRAMMAR/DIAGNOSTICS` now pub
- tests: 317 passed (mcp unit tests; compiler/tests/mcp.rs handshake, tools, sandbox: absolute prefix, empty cwd, timeout)
- decisions: ADR-0030 proposed
- next: T-026 `aleph tokens` (ADR-0031)
- agent: orchestrator (claude-opus-5.5) directly, OpenCode quota exhausted; not independently reviewed

## 2026-11-16T18:15 T-026 done
- `aleph tokens FILE [--tokenizer NAME|all] [--json]` (new `tokens.rs`, ADR-0031): table of counters chars4 (default, ceil(chars/4)), bytes, chars, words, aleph (lexer tokens; lex errors reported as diagnostics); no BPE file in repo, so no real model tokenizer
- unknown tokenizer: usage error exit 2; MCP `tokens` tool (default all)
- `every_stubbed_subcommand_reports_not_implemented` replaced by `tokens_missing_file_reports_x002`: no subcommand is stubbed any more
- tests: 322 passed
- decisions: ADR-0031 proposed
- next: M3 complete; independent DeepSeek review of T-024..T-026 when OpenCode quota returns; v0.2 not started
- agent: orchestrator (claude-opus-5.5) directly, OpenCode quota exhausted; not independently reviewed

## 2026-11-16T20:00 T-027 done
- `dict K V` (ADR-0032): lexicon type `dict`; parser type arity 2 and expression `dict K V` = empty dict (type name as constructor); AST `TypeKind::Dict`, `ExprKind::Dict`; printer; checker `Type::Dict`, key type i64/bool/str else T004 (expected `key`), `Bound::Key`
- pure builtins put/get/del/keys/vals/size (names reserved); not showable/primitive/comparable
- interpreter: `Value::Dict` sorted pair vector, binary search; Tlön: `AL_DICT` tag reusing `AlList` storage (k0 v0 k1 v1 ...), `aleph_dict_empty`, `aleph_b_put/get/del/keys/vals/size`; cgen emits `aleph_dict_empty()`
- SPEC §4/§5.2 and grammar.ebnf updated (marked v0.2, ADR-0032 Proposed); llms.txt: dict in §4, builtin list compacted to `name sig [! cap]` lines to stay within 1950 tokens (7795 chars); digests updated
- goldens: run_dict_words, run_dict_ops (.out, interpreter and native identical via differential test), dict_f64_key/dict_show/dict_eq (.diag T004)
- tests: 322 passed (golden/differential files counted per harness test)
- decisions: ADR-0032 proposed
- next: T-028 JSON (Proposed ADR first)
- agent: orchestrator (claude-opus-5.5) directly, OpenCode quota exhausted; not independently reviewed

## 2026-11-16T21:40 T-023..T-027 review fixes
- independent reviews by Claude Opus 5.5 subagents (read-only, fresh context): T-023 7/10, T-024 6/10, T-025 7/10, T-026 8/10, T-027 8/10
- f53d03f (T-024): fn replace with multi-line text works (was always X010); U+0001 in text rejected; markers substituted left to right; expression fragments must be one expression (`[fragment]` probe + balanced brackets); `with` alias so fix objects are edits
- 49e6c5c (T-025): non-UTF-8 line -> -32700, session continues; envelope checks (jsonrpc, null/object id); initialize answers only 2025-06-18; temp dir 0700; Linux child PR_SET_PDEATHSIG + RLIMIT_AS 1 GiB; args > 100 KiB -> -32602
- c71af00 holds three fixes (the commit helper staged everything; its message names only T-023): T-023 no capability fix for paths normalizing to the whole dir (`.`, `./`); T-026 `tokens --tokenizer all` keeps other counts when the lexer fails (`-`/null); T-027 Tlön test_dict, golden run_dict_order, ADR-0032 notes (native O(n²) memory, display-only dict text in E002)
- not changed (minor, noted): F001 carries the canonical text twice (needs a SPEC §9 decision); llms.txt fix example not restored (token budget 1949/1950); `show` spelled as a literal in check.rs like everywhere else builtin names are used
- tests: 329 passed
- next: second review round of the fixed commits; then T-028 JSON

## 2026-11-16T22:30 T-023..T-027 review rounds 2-3
- round 2 (fresh Opus reviewers): T-023 8, T-024 8, T-025 8, T-026 9, T-027 8, minors only -> 8b1420b (MCP: 64-bit Linux gate, variadic prctl, RLIMIT_FSIZE, total arg cap, NUL, no reply to responses), ffb2a6f (edit: canonical ids, probe found, fix objects via edit in tests/fixes.rs), 1e52eea (one fix per use line), bdac95f (dict value type errors with a bad key)
- round 3: T-023 8 (IMPORTANT: has/isok fix on bool payload), T-024 8.5, T-025 8 (IMPORTANT: current_exe after rebuild), T-027 9 -> a1f4f21 (/proc/self/exe, `signal`, -32602 for bad timeout/file/absolute prefix), 3fd8369 (blank text X010, big ids X008), 8f04177 (no has/isok fix on bool payload)
- passed (>=9, no CRITICAL/IMPORTANT): T-026, T-027. Review cap (3 cycles) reached for T-023/T-024/T-025; round-3 fixes not re-reviewed
- tests: 330 passed
- next: optional 4th review round (user decision); T-028 JSON

## 2026-11-17T09:30 T-028 done
- `json` type (ADR-0033): lexicon type `json`, parser/AST `TypeKind::Json`, checker `Type::Json`, `Bound::Jsonable`; `show` accepts json (compact JSON, keys ascending)
- pure builtins `json x` (unit/bool/i64/f64/str/list json/dict str json; NaN/inf -> null), `jparse` (RFC 8259, i64 integers else f64, depth 256, duplicate keys rejected, errors `json: <reason> at byte <n>`), accessors `jbool jint jnum jstr jlist jdict` -> opt
- interpreter: `compiler/src/json.rs` (parser/printer), `Value::Json(Rc<Value>)`; Tlön: `AL_JSON` boxed payload, same parser in C (malloc'd scratch arrays, no arena garbage while parsing); the C test `test_json` uses the same case table as the Rust unit tests
- SPEC §4/§5.1/§5.3, grammar, lexicon; llms.txt (2036 est. tokens): ADR-0026 amended, budget 1950 -> 2050 (v0.2 builtins do not fit; user may want to revisit)
- goldens: run_json (.out), run_json_error (.out/.err/.exit, differential), json_eq/json_bad_value/json_key (.diag T004)
- tests: 333 passed
- decisions: ADR-0033 proposed; ADR-0026 amended
- next: independent review of T-028; then T-029 sum types + match
- agent: claude-opus-5.5 directly (OpenCode quota exhausted)

## 2026-11-17T11:00 T-028 review round 1
- independent review (fresh Claude Opus subagent, sanitizers + 400-case fuzz, no engine divergence): 7/10, IMPORTANT: object parse quadratic in keys (100k descending keys 26 s); program-built json nested without limit (interpreter stack overflow vs native segfault)
- fixes: members collected and sorted once per object (duplicate key reported after the object parsed, smallest repeated-key offset); json values carry their depth, `json` traps R005 above 256 in both engines; `json` of a `json` returns it; ADR-0033: truncated `\u` = bad escape
- tests: Rust/C case tables extended (-0.0, 1e15/1e16, \u0000, \/, duplicate order, 200k-key object), goldens run_json_deep (.err/.exit R005, differential) and json_example_fail (E002 on json)
- tests: 335 passed; llms.txt 2042 est. tokens (budget 2050)

## 2026-11-17T11:40 T-028 review round 2
- fresh Opus reviewer (sanitizers, ~85 edge inputs, hostile sizes, line-by-line Rust/C comparison): 8.5/10, no CRITICAL/IMPORTANT, no parity break
- minors fixed: AlJson embeds AlBox (strict aliasing); ADR-0033 states -0 -> 0, big integers -> f64, jint of 1.0 none, R005 reuse for depth; golden run_json_file (jparse on file input incl. number edges, raw tab, duplicates, both engines)
- tests: 335 passed

## 2026-11-17T13:30 T-028 review round 3
- Opus review 8/10: one IMPORTANT finding. Numbers with ~1M digits and huge exponents parsed differently, because Rust's f64 parser stops at exponents past ~65536 and strtod does not. Fix: both engines reject number tokens longer than 1024 bytes as `bad number`.
- minor: examples compared `json` with IEEE `==` (`0.0` = `-0.0`); now compared by printed text. Golden json_negzero.
- minor: `show` on JSON with shared subtrees can be exponentially large. Documented as a known limit in ADR-0033, not capped.
- tests: 335 passed, clippy clean
- next: review cap (3 cycles) reached; the round-3 fixes were not re-reviewed. Next task: T-029.

## 2026-11-17T15:30 T-029 done
- sum types and `match` (ADR-0034): `type NAME | V F.. | ..`, variants are constructors (arity = field count) returning the sum; `match e ARM..` with one arm per variant in declaration order (`V b1..bk body`), exhaustive by construction
- lexer `|` (TokenKind::Bar), lexicon keyword `match` and punctuation `bar`; AST `TypeDef.variants` (Variant nodes), `ExprKind::Match` with `Arm {variant, binders, body}` (no id); ids pre-order (match, scrutinee, arm bodies); printer round-trips
- parser: pass 1 registers variants as constructors plus sum/variant maps; clashes P001; `match` P001 for wrong/other/missing arms and non-ident binders, T005 for shadowing binders
- checker: sum names lower to `Type::Record(sum)`, variants live in `records` (constructor fields); scrutinee checked against the sum (T001), expected type flows into the first arm, its type into the rest (as `if`); `==`/`show`/dict key give T004, `.f` T003
- interp: variant = `Value::Record(variant, fields)`, match clones env with binders; cgen: one `AlRecordInfo` per variant, match = if/else chain on `rec->info` (last arm plain `else`), binders declared in the arm block, fast path unboxes binders; no Tlön change
- explain text/JSON (`variants` only on sums), SPEC §2/§3.2/§4/§5, grammar, llms.txt (2050 est. tokens = budget; prose trimmed, budget unchanged)
- goldens: run_sum_tree, run_sum_expr, run_sum_deep (.err/.exit R004 through match + try in an arm near the depth limit, both engines); diag sum_arm_order, sum_arm_other, sum_scrutinee, sum_arm_types, sum_binder_shadow, sum_variant_dup, sum_variant_fn, sum_variant_type, sum_eq, sum_example_fail, sum_field
- tests: 344 passed, clippy clean
- decisions: ADR-0034 (Proposed, amended: P001 for name clashes, T003 for `.f`)
- next: independent review of T-029; llms.txt is exactly at the 2050 budget, the next feature needs trimming or a budget decision; then T-030
- agent: claude-opus-5.5 directly

## 2026-11-17T14:30 T-029 review round 1
- Opus review 8.5/10, minors only. Fixed:
  - ast golden `ast_sum`;
  - P001 at a wrong arm now carries `expected` (the variant due there);
  - llms.txt restores "builtins generic, user fns not", still within 2050;
  - the interpreter skips the env copy for arms without binders.
- Not changed, noted: linear dispatch over variants (fine at 500 variants); nullary variants allocate per construction (same as empty records).
- tests: 344 passed, clippy clean

## 2026-11-17T15:30 T-029 review round 2
- Opus review 8.5/10.
- IMPORTANT, fixed: the random-AST round-trip test generated no sums or `match`. It now generates a `shape` sum, variant constructors, and `match` with unique binders, and asserts that more than 100 `match` expressions were generated.
- minor, fixed:
  - generated C marks binders `(void)`, so no unused-variable warnings;
  - wording is now "expected variant name" everywhere.
- minor, not changed: env clone per arm with binders (interp), allocation of zero-field variants.
- new task T-034 for a pre-existing clash between user `fn kind` and Tlön's `aleph_fn_kind`.
- tests: 344 passed, clippy clean

## 2026-11-17T16:30 T-029 review round 3
- Opus review 9/10, no CRITICAL or IMPORTANT findings, so the target is met.
- minors fixed:
  - llms T005 rule names binders;
  - explain_sum golden (text and JSON);
  - P001 on a non-variant first arm carries `expected: variant name`;
  - check resolves arm types like `if` does.
- not done: no `aleph edit` unit test that touches `match`. It works when tried by hand (X009 on variant ids).
- tests: 344 passed, clippy clean
- next: T-030 scoped regions (ADR-0007), T-031, T-034

## 2026-11-18T16:30 ADR-0036 builtins done (by Bernardo)
- builtins `abs min max starts ends uniq map2` (ADR-0036): table, bound `numeric`, interpreter, Tlön runtime, SPEC §5.1, lexicon, llms.txt
- goldens: run_builtins_adr36, run_trap_abs, type_t004_abs, ex_builtins_adr36; runtime tests test_adr36
- ADR-0026 amended: llms.txt token budget 2050 -> 2110
- why: token pilot of 18 Nov (`experiments/pilot/tokens/` on exp/protocols): missing builtins become typed helpers; HumanEval median Aleph/Python 1.23 -> 1.08 with these builtins
- tests: all passed, clippy clean
- next: ADR-0035 is left for T-044 (ord/chr); inline lambdas are the bigger lever (1.08 -> 0.81) and get their own ADR

## 2026-11-19T08:00 T-034 done
- Tlön: `aleph_fn_kind`/`aleph_fn_static` renamed `aleph_fnval_*`. User names never contain `_`, so generated `aleph_fn_<name>` can no longer clash with the runtime.
- golden run_fn_names (user fns `kind` and `static`, interpreter and native)
- tests: 344 passed, clippy clean
- next: T-045..T-049 lambdas (ADR-0037)

## 2026-11-19T12:00 T-045..T-047 done
- keyword `lam` (lexicon.toml, lexicon.rs); AST `ExprKind::Lam { binders, body }` (ids pre-order: lam node, then body); parser takes the binder count from the builtin's function-typed parameter (map/filter 1, fold/map2 2), binders follow T005 like match binders; `lam` elsewhere is P001 with `expected`; printer `lam x y BODY`; round-trip generator emits lambdas
- check: lambda arguments are checked after the others, binders typed from the instantiated signature, body against the result type; `try` in a lambda body is T007; capabilities/effects via the shared walkers
- interp: `FnTarget::Lam(node id, captured env)`, captures by value only the outer names the body reads; map/filter/fold/map2 call it through `call_target`
- cgen: `aleph build` with `lam` in a let/do/result line is X001 "not implemented" at the first lambda (no C emitted) until T-048
- goldens: lam_forms (ex lines only, no .out yet because of the differential test), ast_lam, lam_outside/binder/shadow/shadow_outer/filter_type/effect/try .diag
- SPEC §2/§5 and grammar.ebnf updated, llms digests and prose; llms.txt 2110 tokens (budget 2110, trimmed "Canonical, check-clean" to fit)
- tests: 353 passed, clippy clean
- decisions: ADR-0037 amendments (try in lambda T007, P001 expected text, binder scoping, capture of read names only, build X001)
- next: T-048 native lambdas (lift `ExprKind::Lam` to static C fns; captured values = free Vars of the body); then add .out goldens with lambdas

## 2026-11-19T18:00 T-048..T-049 done
- tlon: `AlFn` = `{code, name, kind, captured, ncaptured}`, `AlFnCode` = `AlValue (*)(const AlFn *, AlValue *, size_t)`; map/filter/fold/map2 call `fn->code(fn, args, n)`; `aleph_fnval_lam` (kind `AL_FN_LAM`, captures copied into the arena); inline `aleph_lam_enter/leave` mirror `interp::call_lam` (check call+eval depth at the current location, then call depth only); test_tlon.c updated (+ lambda capture checks)
- cgen: X001 gate removed; each `lam` lifted to `aleph_lam_n<id>` (fast, unboxed binders/captures, falls back to cold `aleph_lamslow_n<id>`); captures = body reads minus names bound inside, sorted; lambda bodies excluded from the enclosing frame (temps, `max_eval_depth`); ref wrappers take `aleph_self`, static `AlFn` initializers get `NULL, 0`
- explain: `lambdas: a function of x [and y] returning BODY` lines / `lambdas` JSON array, only when present (lambda-free output unchanged)
- edit: lam and lam body are expression nodes; fragments in a higher-order function position are probed as `[CALLEE fragment 0..]`
- goldens: lam_native.out, lam_trap_r001/r002/r004 (.out/.err/.exit), explain_lam; unit tests cgen/edit, mcp lambda test
- R004 thresholds through lambdas checked equal in both engines (3 recursion shapes, 12 depth alignments)
- timing (debug interp): map with capturing lambda over 1M: interp 2.14s, native 0.02s; 2M: 4.34s / 0.05s (linear)
- llms.txt/SPEC unchanged (nothing native-specific was missing)
- tests: 356 passed, clippy clean
- decisions: ADR-0037 amendments (runtime layout, lifting, depth accounting, explain format, edit probe)
- next: T-044 ord/chr; re-count the HumanEval token corpus with lambdas (ADR-0037 consequence)

## 2026-11-19T12:00 T-045..T-049 review round 1
- Opus review 8.5/10.
- IMPORTANT, fixed: `ok`/`err` in a lambda body, or as fold's init, gave T010. A builtin call with a lambda now unifies the expected type with the builtin's result type first. Golden lam_ok, run in both engines.
- minor, fixed: a lambda with no captures is one static AlFn (no arena allocation per evaluation).
- minor, noted: the llms wording "Canonical, check-clean" became "Check-clean" in fb8c7f7, to fit the budget.
- tests: 356 passed, clippy clean

## 2026-11-19T13:00 T-045..T-049 review round 2
- Opus review 8/10.
- IMPORTANT, fixed: the early expected-type unification (round-1 fix) moved mismatch errors into lambda bodies. Builtin calls with a lambda are now checked in this order:
  - plain arguments;
  - the expected type;
  - `ok`/`err`/`none` arguments;
  - lambdas.
- Goldens lam_mismatch_fold and lam_mismatch_filter: one T001 at the call. lam_ok still passes.
- minor, fixed: the SPEC `lam` row states the order.
- minor, fixed: new task T-050 for the token recount the ADR requires.
- minor, not changed:
  - the interpreter recomputes captures on every lambda evaluation (about 40% slower than `@f` in debug);
  - two static AlFn per capture-free lambda (fast and slow body; correct);
  - edit message for a bare `lam`;
  - an extra T010 after T007 for `try` in a lambda.
- tests: 356 passed, clippy clean

## 2026-11-19T14:30 T-044 done
- builtins `ord` (str -> opt i64, scalar value of the first character) and `chr` (i64 -> opt str, none outside Unicode scalar values) in builtins, interp, Tlön, lexicon, SPEC §5.1 and llms.txt
- golden run_ord_chr covers ASCII, 2-, 3- and 4-byte characters, surrogates and limits, chr 0, and a lambda shift (interpreter and native)
- llms.txt: dropped the round-trip invariant sentence from §10 to stay within 2110 tokens
- tests: 356 passed, clippy clean
- decisions: ADR-0035

## 2026-11-19T15:00 T-045..T-049 review round 3
- Opus review 8/10.
- IMPORTANT, fixed: `ok`/`err`/`none` nested inside a non-lambda argument (`[ok 0]`, `if`, a nested lambda call) gave T010 with `lam` but not with `@f`. Any argument that contains one now waits for the expected type. Golden lam_ok_nested covers the three forms; the mismatch goldens are unchanged.
- minor, documented in the ADR: an `ok` element type that only a lambda body could supply is still T010.
- minor, not changed: a misleading first T001 when a binder's type is unknown (no expected type).
- The review cap (3 rounds) is reached; the round-3 fix was not re-reviewed.
- tests: 356 passed, clippy clean
- next: T-050 token recount, or the experiments tasks T-035..T-043
