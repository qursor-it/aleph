# Aleph language specification — v0.1 (draft)

Status: draft. This document is the authority for implementation. Where it is silent or ambiguous, follow `AGENTS.md` (write a Proposed ADR, make the smallest decision, move on).

Keywords MUST, MUST NOT, SHOULD follow RFC 2119.

Aleph is a programming language whose primary authors and readers are AI agents. Goals: few tokens, native speed, correctness on the first attempt, safe by default. Human readability is NOT a goal of the source text; humans read the generated explanation (`aleph explain`).

Component names (see ADR-0014): **Aleph** = language + compiler. **Tlön** = runtime (where programs live). **Uqbar** = registry of verified definitions (by hash). **Orbis** = corpus generation pipeline.

---

## 1. Source files

- Extension: `.aph`.
- Encoding: UTF-8. Outside string literals only printable ASCII (0x20–0x7E) and LF are allowed.
- Line endings: LF only. No tabs. No trailing spaces. No empty lines. File ends with exactly one LF.
- One file = one module (modules/imports: v0.2).

## 2. Lexical structure

Tokens are separated by exactly one space. Line structure is significant:

- A line starting at column 0 starts a top-level item.
- A line starting with exactly one space is a body line of the preceding `fn` item.

Token classes:

| Class | Form | Notes |
|---|---|---|
| ident | `[a-z][a-z0-9]*` | no `_`, no `-`, no uppercase. Keywords are reserved. |
| int | `0` or `-?[1-9][0-9]*` | `-5` (no space) is a literal; `- a b` is subtraction. |
| float | `-?[0-9]+\.[0-9]+` | |
| string | `"..."` | escapes: `\n \t \" \\`. No other escapes. |
| fnref | `@ident` | reference to a function as a value |
| field | `.ident` | field access operator, arity 1 |
| param | `ident:` | in fn headers, immediately followed (no space) by a type |
| punct | `[` `]` `=` `->` | `[` and `]` attach to the first/last list element: `[1 2 3]` |
| op | `+ - * / % == != < <= > >=` | see §5 |

Keywords (reserved, see `lexicon.toml`; the spellings are provisional and will be optimized against real tokenizers — ADR-0005):

`fn type use let do ex doc if try some none ok err else has isok empty nil true false and or not neg`

## 3. Top-level items and canonical order

A file is a sequence of items in this canonical order (ADR-0003):

1. `use` items, sorted lexicographically by full line text.
2. `type` items, sorted by type name.
3. `fn` items, sorted by name, except `main`, which is always last.

### 3.1 Capabilities: `use`

```
use fs.read "PATHPREFIX"
use fs.write "PATHPREFIX"
use net.host "HOST"      (v0.2)
use env                  (v0.2)
use time                 (v0.2)
```

Default is deny (ADR-0008). Writing to stdout/stderr is always allowed. Path prefixes are relative to the working directory, compared after normalization (no `..` allowed in prefixes).

### 3.2 Record types: `type`

```
type point x:f64 y:f64
```

- Defines a record `point` with fields `x`, `y`.
- Constructor: the type name used as a function, arity = number of fields: `point 1.0 2.0`.
- Field access: `.x p`.
- Sum types and user generics: v0.2.

### 3.3 Functions: `fn`

Header:

```
fn NAME PARAM* -> TYPE
PARAM := ident ":" TYPE        (no space around ":")
```

Body lines, each starting with one space, in this canonical order:

1. `doc "STRING"` — optional, at most one. Intent annotation (ADR-0003, §4.10 of the design doc). Does not affect execution.
2. `ex EXPR = EXPR` — examples. MUST be at least one for every function except `main` and functions that call effectful builtins. Examples are executed by `aleph check`/`aleph test` (ADR-0010).
3. Statements, any number:
   - `let NAME EXPR` — immutable binding. Shadowing is an error (T005).
   - `do EXPR` — evaluate for effect; EXPR must have type `unit`.
4. Result: exactly one final line `EXPR`, the value of the function.

`main` has no parameters and returns `unit` or `res unit str`. If it returns `err e`, Tlön prints `e` to stderr and exits with code 1.

## 4. Types

```
TYPE := i64 | f64 | bool | str | unit
      | list TYPE
      | opt TYPE
      | res TYPE TYPE
      | ident                      (user record type)
```

Type syntax is prefix with fixed arity, like expressions: `res list i64 str` = result of (list of i64) or str.

- Static typing, bidirectional checking. Function signatures are always explicit; `let` types are inferred.
- No implicit conversions, no subtyping, no overloading of user functions.
- Builtins may be generic (e.g. `len` over any `list T`); user functions are monomorphic in v0.1.
- `dict K V`, JSON value type, sum types, user generics: v0.2.

## 5. Expressions

All expressions are prefix with fixed arity. There are no parentheses. The parser is arity-directed: it MUST first collect all function signatures and record types of the file (pass 1), then parse bodies (pass 2).

| Form | Arity | Type rule |
|---|---|---|
| literal int/float/string, `true`, `false`, `nil` | 0 | i64 / f64 / str / bool / unit |
| variable `x` | 0 | type of binding |
| `@f` | 0 | function value (only as argument to higher-order builtins) |
| `f a1 .. an` | n = declared arity | user fn, builtin, or record constructor |
| `+ - * / %` | 2 | both i64 or both f64; result same. `%` i64 only |
| `== !=` | 2 | same primitive type (i64, f64, bool, str); result bool |
| `< <= > >=` | 2 | both i64 or both f64; result bool |
| `and or` | 2 | bool, short-circuit |
| `not` | 1 | bool |
| `neg` | 1 | i64 or f64 |
| `if c a b` | 3 | c: bool; a, b same type |
| `[e1 .. en]` | n≥1 | all same type T; `list T` |
| `empty T` | 1 (type) | `list T` |
| `.f e` | 1 | e: record with field f |
| `some e` | 1 | `opt T` |
| `none T` | 1 (type) | `opt T` |
| `ok e` / `err e` | 1 | `res T E`; the other type parameter comes from the expected type. If no expected type is known: error T010 |
| `try e` | 1 | e: `res T E`, enclosing fn returns `res U E`; yields T or returns `err` early |
| `else e d` | 2 | e: `opt T` or `res T E`, d: T; yields T |
| `has e` | 1 | e: `opt T`; bool |
| `isok e` | 1 | e: `res T E`; bool |

Evaluation is strict, arguments left to right (ADR-0009).

### 5.1 Builtins (v0.1)

Pure:

| Name | Signature |
|---|---|
| `len xs` | list T -> i64 |
| `at xs i` | list T, i64 -> opt T |
| `push xs x` | list T, T -> list T (new list) |
| `slice xs a b` | list T, i64, i64 -> list T (clamped) |
| `rev xs` | list T -> list T |
| `sort xs` | list T -> list T (T primitive, ascending) |
| `range a b` | i64, i64 -> list i64 (a inclusive, b exclusive) |
| `sum xs` | list i64 -> i64 |
| `map @f xs` | (A -> B), list A -> list B |
| `filter @f xs` | (A -> bool), list A -> list A |
| `fold @f init xs` | (B, A -> B), B, list A -> B |
| `cat a b` | str, str -> str |
| `count s` | str -> i64 (bytes) |
| `split s sep` | str, str -> list str |
| `join xs sep` | list str, str -> str |
| `trim s` | str -> str |
| `upper s` / `lower s` | str -> str (ASCII) |
| `contains s sub` | str, str -> bool |
| `show x` | primitive or list of primitives -> str |
| `int s` | str -> res i64 str |
| `float s` | str -> res f64 str |
| `args` | -> list str (command line arguments; always allowed) |

Effectful:

| Name | Signature | Capability |
|---|---|---|
| `print s` | str -> unit | none (stdout always allowed) |
| `eprint s` | str -> unit | none |
| `read path` | str -> res str str | `fs.read` covering path |
| `write path s` | str, str -> res unit str | `fs.write` covering path |

`fetch`, `now`, `env`, `json`, `dict`: v0.2.

## 6. Runtime semantics (Tlön v0.1)

- Values are immutable. Lists have value semantics (an implementation may share structure).
- i64 overflow, division or modulo by zero: trap. f64 follows IEEE 754.
- Trap = structured runtime error on stderr (JSON, §9) and exit code 70.
- Recursion is allowed. Loops: use `range`/`map`/`filter`/`fold`. (General loops: v0.2.)
- No undefined behavior: every operation either succeeds or traps.

## 7. Memory (Tlön v0.1)

- No garbage collector (ADR-0007).
- v0.1: a single arena per program run; nothing is freed before exit. This is deliberately simple.
- v0.2: scoped regions per call with explicit escape of returned values.

## 8. Capabilities (ADR-0008)

- The checker MUST reject a call to an effectful builtin if the module lacks the required `use` (C001).
- If the path/host argument is a string literal, the checker MUST verify it is covered by a declared prefix (C002). Otherwise Tlön checks at run time and traps with R003 on violation.
- `aleph explain` MUST list all capabilities of a program.

## 9. Diagnostics (ADR-0011)

Every tool emits diagnostics as JSON when called with `--json`, one object per line:

```json
{"code":"T012","severity":"error","file":"a.aph","line":3,"col":14,"node":"n42","msg":"type mismatch","expected":"i64","found":"str","fix":{"op":"replace","node":"n42","with":"int n42"}}
```

- Required: `code`, `severity`, `file`, `line`, `col`, `msg`.
- Optional: `node`, `expected`, `found`, `fix`, `canonical` (for F codes: the canonical text of the line or file).
- `msg` is short, lowercase, no trailing period.
- Code prefixes: `L` lexical, `P` parse, `F` format/canonical, `T` type, `C` capability, `E` example failed, `R` runtime trap.
- `fix` is optional and MUST only be emitted when the fix is certain.

Initial codes (extend as needed, keep a table in `spec/diagnostics.md`):

| Code | Meaning |
|---|---|
| L001 | invalid character |
| L002 | invalid token |
| P001 | unexpected token |
| P002 | wrong number of arguments (arity) |
| P003 | unknown function |
| F001 | not in canonical form (include `canonical`) |
| T001 | type mismatch |
| T005 | shadowed binding |
| T010 | cannot infer type of ok/err/none |
| E001 | missing examples |
| E002 | example failed (include expected and actual) |
| C001 | missing capability |
| C002 | path not covered by capability |
| R001 | integer overflow |
| R002 | division by zero |
| R003 | capability violation at run time |

## 10. Canonical form (ADR-0003)

- The AST is the source of truth; the text is its canonical printing.
- `aleph fmt` prints the canonical form of any parseable file.
- `aleph check` (default strict) rejects any file whose text differs from its canonical printing (F001) and includes the canonical text.
- Invariant (MUST be tested): `print(parse(t)) == t` for every canonical `t`, and `parse(print(ast)) == ast` for every AST.

## 11. Command line (`aleph`)

| Command | Behavior |
|---|---|
| `aleph check FILE [--json]` | lex, parse, canonical check, types, capabilities, run examples |
| `aleph fmt FILE` | print canonical form to stdout (`-w` to write in place) |
| `aleph test FILE [--json]` | run examples only |
| `aleph run FILE [ARGS..]` | run with the interpreter |
| `aleph build FILE -o OUT` | compile via C backend and system C compiler |
| `aleph explain FILE [--json]` | deterministic summary: capabilities, types, function signatures, docs, examples, call graph |
| `aleph ast FILE` | AST as JSON with node ids |
| `aleph tokens FILE` | (v0.2) token counts per configured tokenizer |

Exit codes: 0 ok, 1 diagnostics with errors / program returned err, 2 usage error, 70 runtime trap.

## 12. Examples

```
fn add a:i64 b:i64 -> i64
 ex add 2 3 = 5
 + a b
fn fib n:i64 -> i64
 doc "nth fibonacci number"
 ex fib 0 = 0
 ex fib 10 = 55
 if < n 2 n + fib - n 1 fib - n 2
fn total xs:list i64 -> i64
 ex total [1 2 3] = 6
 ex total empty i64 = 0
 fold @add 0 xs
fn main -> unit
 print show fib 20
```

```
use fs.read "data"
fn words s:str -> i64
 ex words "a b c" = 3
 len split trim s " "
fn main -> res unit str
 let text try read "data/in.txt"
 do print show words text
 ok nil
```

Note on `ex words "a b c" = 3`: `words` has arity 1, so the left expression ends after `"a b c"`; `=` separates the two sides.

## 13. Out of scope for v0.1 (planned)

Modules/imports, `dict`, JSON, sum types, generics, lambdas, general loops, network, time, env, scoped regions, LLVM and WASM backends, content addressing (Uqbar), MCP server, tokenizer measurement. See `TASKS.md` for order.
