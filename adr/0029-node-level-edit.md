# ADR-0029: Node-level edits (`aleph edit`)

Status: Proposed
Date: 2026-11-16

## Context
ADR-0003 makes the AST the source of truth and gives every node a stable
pre-order id so that agents can edit by node instead of by text. T-024 asks
for `aleph edit FILE --json EDITS` with replace/insert/delete by node id. The
spec does not define the JSON shape, the allowed node kinds or the errors.

## Decision
- `EDITS` is a JSON array of objects `{"op","node","text"?,"position"?}`.
  Unknown fields are rejected. `node` is an id such as `n12`. `with` is
  accepted as an alias of `text`, so a diagnostic's `fix` object
  (ADR-0028) is a valid edit as is.
- All ids refer to the **input** file (parsed and canonicalized as by
  `aleph ast`). Edits are applied together against that snapshot; their
  order matters only for several inserts at the same anchor.
- Operations and allowed node kinds:
  - `replace` + `text`: file `n0` (whole text), `use`/`type`/`fn` item (item
    text, a `fn` may span lines), `ex`/`let`/`do` line (one body line,
    without the leading space), expression (one-line canonical-style
    expression text).
  - `delete`: `use`/`type`/`fn` item, `ex`/`let`/`do` line.
  - `insert` + `text`: at `n0` adds a top-level item; at a `fn` adds a body
    line, `position` `start` (after header and `doc`) or `end` (default,
    before the result line); at an `ex`/`let`/`do` line, `position`
    `before` or `after` (default).
  Params, fields and types are not editable directly; replace the enclosing
  item.
- The edited text is reparsed and printed canonically, so inserted items land
  in canonical order (SPEC §3). Nothing else is checked: types, capabilities
  and examples are `aleph check`'s job.
- Output: the edited canonical text on stdout, exit 0. With `-w` the file is
  written atomically (same path as `fmt -w`) and then `aleph check`
  diagnostics of the new file are printed, with ids of the new AST; the exit
  code is then `check`'s (1 on errors), so a written-but-failing edit is told
  apart from a rejected one by the code prefix (`X` = rejected, not written).
- Errors (no output, no write, exit 1):
  - `X007` invalid edits json (malformed JSON, unknown op/position/field,
    bad node id spelling such as `x1` or `n013`, missing or superfluous
    `text`/`position`);
  - `X008` unknown node id (also a well-spelled id beyond u32);
  - `X009` edit not allowed for node kind (`found` is `<op> <kind>`, kind
    one of `file use type fn ex stmt expr other`; `stmt` = a `let`/`do` line);
  - `X010` edit does not parse: a one-line fragment containing a newline,
    any text containing U+0001 (reserved for expression markers), blank
    text anywhere but a `replace` of `n0` (these three carry `node` and a
    short `found` such as `blank text`), an
    expression fragment that is not exactly one expression in place
    (unbalanced brackets, or `[fragment]` is not a one-element list, e.g.
    `7 8` inside a list), or the edited program does not parse. `found`
    holds the first parser diagnostic; its line:col refers to the edited
    text, not the input file (the one-expression check gives code and
    message only);
  - `X011` conflicting edits: an edit whose node lies inside a node that
    another edit replaces or deletes (pre-order ids make each subtree a
    contiguous id range).
- Parse errors in the input file are reported as is.

## Consequences
- Agents can apply `fix` objects (ADR-0028) and their own changes without
  reproducing whole files.
- Edits that produce type errors are accepted on stdout; agents must check.
- Expression replacement is textual inside the printed line, so a fragment
  must be a complete prefix expression; otherwise X010. The one-expression
  check reparses the program once more with each fragment wrapped in `[ ]`.
