# ADR-0032: `dict K V` type and builtins

Status: Proposed
Date: 2026-11-16

## Context
T-027 (v0.2) asks for a `dict K V` type and builtins, with a Proposed ADR
first. SPEC lists `dict` as v0.2 and says nothing else. Constraints: values
are immutable (SPEC §6), output must be deterministic, and the interpreter
and the native backend must agree byte for byte (T-020).

Options for the empty-dict expression (score 1-10):
- The type name as constructor, `dict K V`, like record types are
  constructors: no new keyword, prints as its type. **8**
- A new keyword (e.g. `nodict K V`): one more lexicon entry, unusual word. **5**
- Infer from the expected type (like `ok`/`err`): fails without an expected
  type (T010) and the native backend needs concrete types. **4**

## Decision
- Type `dict K V` (prefix, arity 2; lexicon type concept `dict`). The key
  type `K` must be `i64`, `bool` or `str` (no `f64`: NaN has no useful
  equality); otherwise T004 "type constraint not satisfied" with expected
  `key`.
- Expression `dict K V`: the empty dict of that type. The printer writes it
  as its type.
- Dicts are immutable values; "updates" return a new dict. Keys are kept in
  ascending order: `i64` numerically, `bool` `false` before `true`, `str` by
  bytes. Every observation (`keys`, `vals`) follows that order, so output
  is deterministic.
- Pure builtins (no capability):
  - `put d k v` -> `dict K V` (adds or replaces `k`);
  - `get d k` -> `opt V`;
  - `del d k` -> `dict K V` (unchanged when `k` is absent);
  - `keys d` -> `list K`, ascending;
  - `vals d` -> `list V`, in key order;
  - `size d` -> `i64`.
- Dicts are not `showable`, not comparable with `==`/`!=` and not
  primitive; `show`, `sort` and equality on them are type errors.
- Runtime cost: `put`/`del` copy the dict (O(n)); `get` is O(log n) in both
  engines. Tlön stores a sorted array of key/value pairs in the arena; the
  interpreter uses a sorted `Vec` of pairs searched by binary search.

## Consequences
- Agents get a map without new syntax beyond one type name.
- Building a dict of n entries with `put` is O(n²) time, and in the native
  backend also O(n²) memory: the arena never frees, so every `put` leaves a
  full copy behind (4000 keys ≈ 256 MB; same as `push` on lists). Acceptable
  for v0.2; revisit with scoped regions (T-030) or a persistent tree.
- A dict in a failed example's `expected`/`found` (E002) is shown as
  `dict k1 v1 k2 v2 ...` in key order. That text is for display only: it is
  not Aleph syntax and is ambiguous for nested dicts.
- `put`, `get`, `del`, `keys`, `vals`, `size` and `dict` become reserved
  names (T005).
