# ADR-0023: `aleph explain` output contract

Status: Proposed
Date: 2026-11-15

## Context
SPEC §8 requires `aleph explain` to list all capabilities of a program and
SPEC §11 describes it as a deterministic summary of capabilities, types,
signatures, docs, examples and the call graph. T-016 implements the command for
both text and JSON consumers. Without a fixed layout, small implementation
changes would silently break agents that read the report, so the text shape and
the JSON field names are treated as a contract.

## Decision
- **Pipeline.** `aleph explain FILE [--json]` reads the file (`X002`), then
  runs lex/parse, type check and capability check. It does not require canonical
  text (`F001`) and does not run examples (`E001`/`E002`). Any diagnostics are
  emitted as JSON lines to stdout and the command exits 1. A clean file prints
  the report and exits 0.
- **Ordering.** The report is a `capabilities`, `types`, `functions`, `entry`
  sequence. Capabilities are always first (ADR-0008); they are sorted by
  `(cap, path)` and deduplicated. Types are sorted by name. Functions keep the
  canonical file order (`main` last). Calls and builtins are sorted and unique.
  No output depends on hash-map iteration order.
- **Text format.** One item per line, no blank lines:
  - Each capability is `capabilities: CAP [ "path"]`, or the single line
    `capabilities: none`.
  - Each record is `type NAME [ field:TYPE]...` with declared field order and
    canonical type spelling.
  - Each function starts with its canonical header
    `fn NAME [param:TYPE]... -> TYPE`, followed by indented lines in this
    order: `doc`, `effectful`, `examples`, `calls`, `builtins`.
  - `doc` is quoted and escaped like a source string: `doc: "text"` (with
    `\n`, `\t`, `\"`, `\\` escapes), or `doc: -` when absent. This keeps null
    distinct from the empty doc `""`.
  - `examples` repeats its prefix once per example as
    `examples: LHS = RHS` in canonical expression text, or `examples: -`.
  - `calls` and `builtins` are space-separated sorted names, or `-`.
  - The last line is `entry: main`, or `entry: -` when the file has no
    `main` function.

  Example:

  ```text
  capabilities: fs.read "data"
  type point x:f64 y:f64
  fn add a:i64 b:i64 -> i64
    doc: "sum"
    effectful: no
    examples: add 2 3 = 5
    calls: -
    builtins: -
  entry: main
  ```

- **JSON format.** `--json` prints one pretty object (two-space indent, trailing
  LF) with these field names and order:

  ```json
  {
    "file": "a.aph",
    "capabilities": [{"cap": "fs.read", "path": "data"}],
    "types": [{"name": "point", "fields": [{"name": "x", "type": "f64"}]}],
    "functions": [
      {
        "name": "add",
        "signature": "fn add a:i64 b:i64 -> i64",
        "params": [{"name": "a", "type": "i64"}],
        "ret": "i64",
        "doc": null,
        "effectful": false,
        "examples": ["add 2 3 = 5"],
        "calls": [],
        "builtins": []
      }
    ],
    "entry": "main"
  }
  ```

  `file` is the argument exactly as given. `path` and `doc` are `null` when
  absent; `entry` is `null` when there is no `main`. All type spellings,
  signatures and example strings use the canonical printer.
- **Call graph.** `calls` and `builtins` describe the function body only
  (statements and result), never its `ex` examples: examples are tests and
  would otherwise make every recursive function call itself. `calls` contains
  user functions called directly or referenced with `@f`; constructors are not
  user calls. `builtins` contains builtin calls and `@builtin` references.

## Consequences
- Text and JSON consumers can rely on names, order and null/`-` conventions.
- Any incompatible change to the layout or schema requires a superseding ADR.
- The command stays deterministic and independent of `aleph check`'s stricter
  canonical-form and example requirements.
