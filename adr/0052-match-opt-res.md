# ADR-0052: `match` on `opt` and `res`

Status: Proposed
Date: 2026-11-22

## Context
`match` deconstructs user sum types only (ADR-0034). `opt`/`res` are taken
apart with `has`/`isok` plus `else e DUMMY`, or `try`. Two costs, seen in
the dev probe (experiments/devprobe, 13 runs, 260 answers):
- models write `match X some v BODY none BODY` / `match X ok v BODY err e
  BODY` on their own (run6 `inventory`, run13 `rpn` and `brackets`), the
  Rust/OCaml spelling; today each is P001 "expected variant name";
- without it a value is tested, then unwrapped with a dummy default:
  `if == len xs 0 D` + `let x else at xs 0 0` (61 empty-list tests, 53
  dummy heads in the runs, patterns.md), `if not isok r err E` + `else r 0`.
  A dummy default is a value the program never means, the kind of thing a
  reader must check.

Options (score 1-10):
- **Allow `some`/`none` and `ok`/`err` as match arms**, in that order (the
  order models write and `opt`/`res` are spelled in): `match e some x A none
  B` for `opt T`, `match e ok x A err y B` for `res T E`. No new keyword,
  no new llms.txt line beyond a few words in §5. **8**
- **Either arm order.** Friendlier, but breaks the "arms in declaration
  order" rule that makes canonical form unique. **4**
- **Keep as is.** **3**

## Decision
- `match e some x A none B`: e `opt T`, x: T. `match e ok x A err y B`: e
  `res T E`, x: T, y: E. Bodies same type, as other matches. The first arm
  keyword picks opt or res; the other arm must follow (P001 names it).
  Binders follow T005 like other match binders.
- Both engines identical; canonical form prints the arms in that order.

## Consequences
- `match at xs 0 some x BODY none D` replaces the test plus dummy head;
  `match int t ok n BODY err e D` replaces test then unwrap.
- `match` on a sum is unchanged.
