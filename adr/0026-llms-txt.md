# ADR-0026: Generated `llms.txt` context

Status: Proposed
Date: 2026-11-15

## Context
Agents need a small, stable description of Aleph that fits in a prompt. SPEC
§11 does not define such an artifact, and hand-maintained summaries drift from
the specification. T-022 asks for a compact spec (target: under 2000 tokens)
plus canonical examples, generated from the repository sources.

## Decision
- **Command.** `aleph llms` prints `llms::render()` to stdout and exits 0. It
  takes no `FILE` and no flags; any argument is a usage error (exit 2). `llms`
  is the first command whose file argument is optional. `cli::requires_file`
  keeps the distinction for later file-less commands such as `mcp`.
- **Checked-in output.** The repository root `llms.txt` is byte-identical to
  `aleph llms` output. `compiler/tests/llms.rs` fails with the regeneration
  command when it is stale.
- **Sources.** The module uses `include_str!` for:
  - `spec/SPEC.md`: the purpose paragraph verbatim, and section headings by
    exact number (`1`, `2`, `3`, `4`, `5`, `5.1`, `8`, `9`, `10`, `11`);
  - `spec/grammar.ebnf`;
  - `spec/diagnostics.md`: the code table (the compact list keeps the codes in
    spec order);
  - ADR-0018, ADR-0020, ADR-0021 and ADR-0022, whose decisions the semantics
    and capability text summarize;
  - `tests/aph/*.aph`: the ten examples, embedded verbatim.

  The keyword list comes from `lexicon::keyword_names()` and is checked by
  exact equality. The builtin table is generated from `builtins::BUILTINS`
  (`name`, signature, effect, capability); signatures use the `Ty` tree and
  append the `BuiltinSpec::bounds` text, e.g.
  `sort list T->list T (T primitive)` and
  `show T->str (T primitive or list of primitive)`.
- **Drift detection.** Because the prose is a curated digest rather than a
  mechanical concatenation, `llms.rs` records a hand-written FNV-1a 64 hash of
  each summarized source: `spec/SPEC.md`, `spec/grammar.ebnf`,
  `spec/diagnostics.md`, ADR-0018, ADR-0020, ADR-0021 and ADR-0022. A test
  recomputes each hash, asserts that exact path set, and fails with
  `source X changed: review llms.rs digest, then update hash`. Reviewing the
  prose and updating the hash is an explicit step when a source changes.
- **Content.** One purpose paragraph; compressed file/lexical rules; item and
  canonical body order; types; expression arity/type rules; builtin table with
  bounds; capabilities and runtime traps R001-R005, including the v0.1/v0.2
  capability split, the `range` R005 limit and the recursion/evaluation R004
  limit; a compact semantics block (shadowing T005, hermetic examples,
  `print`/`eprint`, `show`, ASCII `upper`/`lower`, `at`, `slice`, `range`,
  `sort`, `count`, `split`, `int`/`float`, `ok`/`err` T010, `main` error);
  diagnostics
  JSON shape, `fix` and code list; canonical form; command list; exactly ten
  fenced examples.
- **Examples.** `llms_ops`, `run_strings`, `bench_upper`, `run_opt_res`,
  `run_higher_order`, `run_records`, `bench_fib`, `run_eprint`, `run_args`,
  `run_show`. Together they cover records, recursion, `opt`/`res`/`try`,
  `map`/`filter`/`fold`, filesystem read/write, examples, `eprint`, `args`,
  string/list builtins and the boolean/arithmetic operators. `llms_ops.aph` is
  a small canonical golden added for T-022. Every example is checked for
  canonical text, the lex/parse/type/capability/example pipeline and verbatim
  equality with its `tests/aph` source.
- **Token estimate.** No tokenizer exists (T-026), so the test estimates tokens
  as `ceil(chars / 4)` over the rendered UTF-8 text and requires at most 1950,
  leaving margin under the 2000 target. This is a budget check, not a claim
  about any particular tokenizer.
- **Determinism.** `render()` uses only static tables, ordered iteration and
  `include_str!`; it does not read the filesystem or environment and is stable
  across runs.

## Consequences
- Changing a builtin, keyword, diagnostic code or summarized source changes
  the digest/test outcome; the test fails until the prose and hash are
  reviewed and `llms.txt` is regenerated.
- The compact text deliberately omits implementation details, module layout,
  memory strategy and v0.2 plans beyond short markers.
- Regenerate with:

  ```sh
  cargo run --quiet --bin aleph -- llms > llms.txt
  ```

- Any future tokenizer may replace the estimate, but the 2000-token target and
  the checked-in file contract stay.
