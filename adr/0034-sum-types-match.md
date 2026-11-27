# ADR-0034: Sum types and `match`

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-17

## Context
T-029 (v0.2) asks for sum types and `match`, with a Proposed ADR first. SPEC
§3.2/§4 only say "sum types: v0.2". Constraints: prefix syntax with fixed
arity and no parentheses (arity known from pass-1 signatures), one canonical
text per AST (ADR-0003), interpreter and native backend byte-identical
(T-020), few tokens in `llms.txt` (ADR-0026).

Options for the declaration (score 1-10):
- `type shape | circle r:f64 | rect w:f64 h:f64`: one new token `|`, reads
  like ML/Haskell, a record never contains `|`, so the two kinds of `type`
  are told apart by the first token after the name. **8**
- `type shape circle r:f64 rect w:f64 h:f64` (a bare ident starts a
  variant): no new token, but hard to read and to tell from a record. **5**
- A new keyword `sum`: one more keyword in the lexicon for no gain. **6**

Options for `match` (score 1-10):
- `match e ARM..` with exactly one arm per variant, in declaration order,
  each arm `ctor b1 .. bk expr` (k = the variant's field count): the first
  arm's constructor fixes the sum type, so the parser knows how many arms
  and binders follow; exhaustive by construction, no wildcard, one canonical
  order. **8**
- Arms in any order plus a `_` wildcard: needs an end marker or a count,
  several texts per AST, and a new token. **4**
- Only accessors (`isX`, `.field`) without `match`: no binding, partial
  functions or `opt` everywhere. **3**

## Decision
- Declaration: `type NAME | V1 F.. | V2 F.. ..` with at least one variant;
  each variant is an ident followed by zero or more fields `ident:type`
  (as record fields). `|` is a new token, used only here. Example:
  `type shape | circle r:f64 | rect w:f64 h:f64 | dot`.
- A sum type is not a constructor; each variant is a constructor function,
  arity = its field count, returning the sum type: `circle 1.0`, `dot`.
- Names: variant names share the namespace of functions, record types and
  builtins, and also may not equal any type name: a clash is P001
  `duplicate definition` (a builtin or built-in type name: P001 `reserved
  name`), exactly as for other duplicate definitions. Field names are local
  to their variant. Types may refer to themselves and each other (recursive
  types), as records do.
- `match e ARM ..`: `e` has a sum type `S`; there is exactly one arm per
  variant of `S`, in declaration order; an arm is the variant name, one
  binder ident per field, then the arm's expression. Arity: `match` takes the
  scrutinee plus one arm per variant; the first arm's variant name determines
  `S` at parse time. Example:
  `match s circle r * 3.14 * r r rect w h * w h dot 0.0`.
  - Parse errors (P001, unexpected token): a token that is not a variant
    name where an arm starts, a variant of another sum type, a variant out of
    declaration order, a binder that is not an ident.
  - Types: `e` must be `S` (T001 otherwise); every arm has the same type,
    which is the type of `match` (bidirectional: the expected type flows into
    every arm). Binders have the field types; they are in scope only in their
    arm, and a binder that shadows a name is T005 (as `let`).
- Sum values are not primitive, not comparable with `==`/`!=`, not
  showable, not dict keys; `.f` does not apply to them (T003, invalid field
  access, as `.f` on any non-record). Examples
  compare them structurally (same variant, equal fields) and E002 shows them
  as their constructor call, e.g. `circle 1.0`.
- Representation: a variant value is a record whose name is the variant
  name (interpreter `Value::Record`, Tlön `AlRecord` with one `AlRecordInfo`
  per variant); `match` dispatches on that name (native: on the info
  pointer). No new runtime value kind.
- `aleph ast`/`explain`/`edit`: the `type` item gains a `variants` list
  (each with name and fields; empty for records); `match` is an expression
  node with the scrutinee and its arms (variant, binders, body). Node ids
  stay pre-order over the printed text: a variant is a node (id before its
  fields); an arm is not (its binders are plain names, like `let`), so the
  ids run match, scrutinee, then each arm body. `explain --json` adds
  `variants` only to sum types, so record output is unchanged (ADR-0023).
- A binder that shadows is reported by the parser and ends parsing of that
  line (one diagnostic per line, like other pass-2 errors).

## Amendments (T-029 implementation)
- The Decision first said T005 for variant-name clashes "as other duplicate
  definitions" and T001 for `.f` on a sum; existing duplicate definitions
  are P001 and `.f` on a non-record is T003, so the implementation follows
  those codes (text above updated).

## Consequences
- JSON (ADR-0033) and errors can be modelled by agents as user sums; JSON
  itself stays an opaque type until generics exist.
- No generic sums (`opt`/`res` stay builtin); no nested patterns or guards:
  nest `match` or use `if` in an arm.
- `match` and `|` become reserved (lexicon); variant names are reserved like
  record names.
- A `match` over a sum with many variants is long but always exhaustive;
  adding a variant breaks every `match` on that type at parse time, which
  tells agents exactly what to update.
