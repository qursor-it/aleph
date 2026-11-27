# ADR-0045: `cat` also concatenates lists

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-21

## Context
Aleph can append one element (`push xs x`) but cannot join two lists. A dev
probe on 21 Nov (20 small tasks, one DeepSeek session that read only
llms.txt; see `experiments/devprobe/`) failed 4 tasks; 2 of the 4 are this
gap:
- `rotate`: `cat slice xs n len xs slice xs 0 n` — the model assumed `cat`
  works on lists, as `+` does in Python (T001 three times);
- `merge`: with no way to put an element in front, the model built the
  result with `push` and got it reversed (E002).
Without a builtin, joining two lists is `fold @push a b` (or `fold lam acc x
push acc x a b`), which no model in the pilot or the probe found.

Options (score 1-10):
- **`cat` generic: `str,str->str` or `list T,list T->list T`.** The name the
  model already reached for; one line in llms.txt changes; no new name to
  learn or reserve. **8**
- **A new builtin `concat`/`extend`.** Clear, but one more reserved name and
  the probe's `cat` call would still fail. **5**
- **List patterns or cons.** New syntax for one operation. **2**

## Decision
- `cat a b` : `T,T->T` with T `str` or `list U`. For strings unchanged; for
  lists the elements of `a` followed by those of `b`.
- Other argument types are T001 as today (expected `str`, or the list type
  of the first argument).
- Both engines identical (T-020); no new limit (memory as for `push`).
- llms.txt: `cat T,T->T (T str or list)`.

## Consequences
- `rotate`, `merge` and similar programs are one line.
- Programs that use `cat` on strings are unchanged.
