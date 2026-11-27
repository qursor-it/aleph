# ADR-0044: Guard lines `if C E` in a function body

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-21

## Context
The T-054 replay re-checks every recorded model answer with the current
compiler. After ADR-0040..0043 the remaining compile errors are few, and most
of them are one shape: the early-return guard every model knows from Python
(`if n <= 1: return False`), written as body lines:

```
fn isprime n:i64 -> bool
 ex isprime 7 = true
 if <= n 1 false
 if == n 2 true
 if hasdiv 2 n false true
```

Each guard line is an `if` with two arguments, so today it is P002 "wrong
number of arguments". In the 18 Nov pilot this is 7 of the 12 diagnostics
left, in 2 of the 5 failing answers (fewshot `unique`, lite `x_or_y`,
written independently), and the fix a
model must find is to fold the guards into one nested `if` on a single line,
which is harder to read and to edit. Nothing else is wrong in those answers.

Options (score 1-10):
- **A body line `if C E` (exactly two arguments, not the last line) is a
  guard statement: if `C` is true the function returns `E`, otherwise the
  body goes on.** Turns the most common remaining error into the meaning the
  model intended; one rule, no new keyword; statements already run in order
  (`let`, `do`), and `try` already returns early. **8**
- **Better P002 message suggesting the nested `if`.** Still a failed round
  and a longer program. **3**
- **A `ret` keyword (`if c ret e`).** New keyword models do not know; the
  replay answers would still fail. **2**

## Decision
- Grammar: a body statement may be `if EXPR EXPR`, a guard. A body line that
  starts with the keyword `if`, is not the last body line, and whose tokens
  after `if` form exactly two complete expressions is a guard. Any other
  `if` line keeps today's rules (the last line is the result; a three-argument
  `if` that is not last is the same error as today).
- Order: guards are statements, so they may appear anywhere among `let` and
  `do` lines (canonical order of ADR-0003 unchanged: doc, ex, statements,
  result). Bindings made after a guard are not visible in it.
- Typing: `C` is `bool`; `E` checks against the function's result type,
  exactly like the result line.
- Semantics: statements run in order; a guard evaluates `C`; if true the call
  returns `E` and the remaining statements and the result are not evaluated.
  Both engines identical (T-020); in Tlön the early return goes through the
  same epilogue as the result (region exit, ADR-0039).
- AST: new statement kind `guard` with `cond` and `value`; printed as
  ` if C E`; node ids in pre-order as for other statements.
- `aleph explain` renders a guard as "if C, return E".

## Consequences
- The guard style of the recorded answers compiles and means what the model
  meant; the replay is rerun and the rate recorded.
- One more statement form in llms.txt, paid by trimming prose.
- A two-argument `if` on a non-last body line was always an error, so no
  valid program changes meaning.
