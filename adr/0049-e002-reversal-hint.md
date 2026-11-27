# ADR-0049: E002 names a reversed list

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
Date: 2026-11-22

## Context
The most frequent wrong-answer cause in the dev probe (experiments/devprobe)
is a list built in reverse: `push REC x` written as if `push` prepended
(run1 `merge`, run3 and run5 `chunks`, run8 `merge` and `transpose`). The
note "(at end)" on the `push` line in llms.txt (T-069) did not remove it.
The model's own `ex` lines catch every case with `E002`, so an agent that
runs `aleph check` learns that the example failed, but `expected [1 2 3]
found [3 2 1]` leaves the cause to be inferred.

Options (score 1-10):
- **E002 says so when found is expected reversed** (outer list, or outer
  and every inner list, at least 2 items): `msg` becomes
  `example failed: found is expected reversed (push appends at the end)`.
  No false positive: it states a fact about two values; costs no llms.txt
  text. **8**
- **A warning on `push REC x` where REC calls the enclosing function.**
  Catches the pattern before examples run, but reversing on purpose is
  legal and the warning would fire there. **4**
- **A prepend builtin.** `cat [x] xs` already prepends; another name costs
  llms.txt budget and the habit is in the reading of `push`, not a missing
  operation. **2**

## Decision
In `aleph check`/`aleph test`, when both example sides evaluate and the
expected value is a list of >= 2 items whose reversal (or reversal of it and
each inner list) equals the found value, `E002` uses the message above.
`expected`/`found` are unchanged. SPEC §9 already leaves `msg` free text.

## Consequences
- One more message string; deterministic; no change for any other E002.
- A reversed palindrome cannot trigger it (values are equal, no E002).

## Evidence after the change
Dev probe run8 repair turn: with these messages the model fixed both list
answers. Control (fresh session, old `example failed` message, same
expected/found): it fixed both, identically. On this evidence the message
is not needed for that model; it costs no llms.txt text and is kept unless
rejected.
