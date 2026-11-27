# ADR-0040: Layout is left to the toolchain (F001 is a warning by default)

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-20

## Context
ADR-0003 makes the AST the source of truth and gives every program one
canonical text; SPEC §10 has `aleph check` reject any other text with F001.
`run`, `test`, `build` and `explain` already skip F001; only `check` enforces
it, and `check` is the command agents use for feedback.

Replaying the recorded pilot answers with the current compiler (T-054): in
the few-shot and lite conditions 6 of 16 answers failed only because of
layout (F001: function order, spacing), all fixed by `aleph fmt` with no
change of meaning. The 18 Nov probe found that rules the model must track
cost reasoning tokens (3,334 → 1,010 median with a shorter spec), and that
rules the compiler can fix mechanically are the cheapest to drop. Layout is
the clearest case: the printer already knows the answer.

Options (score 1-10):
- **`check` reports F001 as a warning with the canonical text and goes on;
  `--strict` keeps it an error.** Agents never fail on layout; the canonical
  text is one `aleph fmt` away and still shown; CI and goldens stay strict.
  llms.txt drops the layout and ordering rules. **8**
- **`check` silently accepts any layout.** Simplest, but agents never see the
  canonical text, and repositories drift from one spelling per program. **5**
- **Keep F001 an error (today).** One spelling enforced at every step, at the
  cost of failed rounds that carry no information about the program. **4**

## Decision
- `aleph check FILE` (and the MCP `check` tool): if the file parses and its
  text differs from its canonical printing, F001 is reported with
  `"severity": "warning"` and the `canonical` text, and checking continues
  (types, capabilities, examples) on the parsed AST. Positions in later
  diagnostics refer to the file as written. Exit code: 0 when there are only
  warnings.
- `aleph check --strict FILE` (MCP: `"strict": true`) keeps today's
  behaviour: F001 is an error and stops checking. The golden tests run
  `check` in strict mode, so their `.diag` files do not change.
- The canonical form, the printer, `aleph fmt` and the round-trip invariants
  of ADR-0003 are unchanged: there is still exactly one canonical text per
  AST, and tools that write files (`fmt`, `edit`) write it.
- llms.txt drops the layout and item-order rules a model no longer needs and
  says that `aleph fmt` prints the canonical form. Rules the parser enforces
  (one leading space for body lines, the order of lines inside a body)
  stay.

## Consequences
- An agent's round fails on layout only in strict mode; the reasoning spent
  on spacing and item order goes away, and llms.txt gains budget.
- Files in a repository can be non-canonical until someone runs `aleph fmt`;
  CI that wants one spelling uses `--strict`.
- SPEC §10 changes: "`aleph check` (default strict)" becomes "warns (F001),
  `--strict` rejects".
