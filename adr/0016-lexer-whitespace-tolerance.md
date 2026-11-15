# ADR-0016: Lexer tolerates non-canonical whitespace

Status: Proposed
Date: 2026-11-15

## Context
SPEC §1 requires canonical whitespace: exactly one space between tokens, no
trailing spaces, no empty lines, LF-only line endings and a final LF. A lexer
that rejected every deviation would make `aleph fmt` useless for repairing
files, because `fmt` must parse the input before it can print the canonical
form. The canonical-form check F001 (T-009) is the component that rejects files
whose text is not already canonical.

## Decision
The lexer accepts these non-canonical whitespace forms:

- multiple spaces between tokens;
- trailing spaces at the end of a line;
- empty lines (they are skipped and do not produce a `Line`);
- a missing final LF;
- multiple leading spaces on a body line: any line starting with at least one
  space is a `Body` line.

Tabs and CR remain `L001` invalid characters outside string literals. All other
canonical-form violations are left to F001, which compares the source text with
the printer output and reports the canonical text.

## Consequences
- `aleph fmt` can read any whitespace-mangled but otherwise well-formed file.
- The lexer's position information stays exact: `col` is the 1-based byte
  column in the original line, including leading indentation.
- Strictness is a separate check (F001); the lexer does not decide what is
  canonical.
- `aleph check` remains strict because it will also run F001.

## Limitation
Whitespace is tolerated only *between* tokens, not inside token forms. A param,
fnref or field must stay attached: `x:T`, `@f` and `.f` are single tokens, while
`x :T`, `@ f` and `. f` produce `L002` invalid-token diagnostics.
