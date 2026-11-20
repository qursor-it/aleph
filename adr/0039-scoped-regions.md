# ADR-0039: Scoped regions as mark/reset with result compaction

Status: Proposed
Date: 2026-11-20

## Context
T-030. ADR-0007 plans, for v0.2, "scoped regions per call; values returned
from a call are copied (or promoted) to the caller region". Today Tlön has
one bump arena per run (64 KiB+ chunks, never freed), so a recursive loop
that allocates temporaries grows memory with its iteration count. Regions
are invisible to Aleph programs: no syntax, no rule, no output change, so
they cost no tokens (the 18 Nov pilot found the cost in rules). The
interpreter frees by reference counting and needs no change.

Constraints: interpreter and native byte-identical (T-020); traps
unchanged; no recursion on value depth (a million-deep sum value already
exists in the goldens); no quadratic copying for functions that return
growing lists.

Options (score 1-10):
- **Mark/reset per user call, compact the result only when it pays.**
  At entry save the arena top; at return, a result whose root is not in the
  call's region (scalars, `none`, values built before the call, static
  values) resets the region for free; a result in the region is copied out
  only when the region is large and the result small, otherwise the region
  is left to the caller. Cheap on the common path, bounded copying. **8**
- **Always copy the result to the caller region (ADR-0007 literally).**
  Simple to state, but a recursive function returning a list of n items
  copies it at every level: O(n²) for list builders that are O(n) today. **4**
- **Escape analysis in the checker to allocate non-escaping values on a
  frame region.** Precise, but a large static analysis for a runtime
  feature. **3**

## Decision
- Scope: every user function call (slow and fast bodies, and the code of
  `@f` values) opens a region; `main`, lambdas and builtins do not.
- Mark: the arena top (current chunk and its `used`). The root check and
  the region size come from one walk over the region's chunks; compaction
  builds a sorted array of the region's chunk ranges once and binary-searches
  it.
- A body that cannot allocate (only unboxed literals, variables, operators,
  `if` and user calls, every node unboxed) gets no region: what it leaves is
  reclaimed by the caller's region. This keeps `fib`-like code as fast as
  before.
- At return:
  1. If the result holds no pointer (`unit i64 f64 bool none`, unboxed fast
     results) or its root object was allocated before the mark (or is
     static), reset to the mark: free the newer chunks (one spare chunk may
     be kept for reuse) and restore `used`.
  2. Else, if the region holds at least 256 KiB and the bytes of the
     result reachable inside the region are at most a quarter of it
     (counted by a bounded traversal that stops past that limit), deep-copy
     those objects into a fresh chunk, reset, and link the chunk as the
     newest one (it belongs to the caller's region). Objects outside the
     region are shared, not copied; shared objects inside it are copied
     once (forwarding through a side hash table keyed by address, so the
     old objects are never overwritten and a failed compaction leaves the
     arena untouched). String data and captured-value arrays are copied with
     the object that owns them. The new chunk is at least 64 KiB and becomes
     the newest one. If an internal `malloc` fails, compaction gives up and
     falls back to rule 3: it never adds a trap.
  3. Otherwise leave the region as it is: the caller's region absorbs it.
- Invariant used: an object allocated before a mark never points to one
  allocated after it (values are immutable; the only builtins that write
  into an older object, `map`/`filter`/`fold`/`map2`, store results of calls
  that already returned).
- Traversal and copy are iterative (explicit stack); they handle every
  `AlTag` (strings and their data, lists, boxes and `AlJson`, records,
  dicts, lambda `AlFn`s with captured values).
- No observable change: same stdout, stderr, exit codes and traps in both
  engines. Out of memory stays a crash as today.

## Consequences
- Peak memory of recursive loops with scalar or small results becomes
  bounded by one iteration instead of growing with the iteration count.
- A tail of costs: two loads and a compare per call on the common path;
  compaction costs at most a quarter of the bytes the region allocated.
- Tlön gains `aleph_region_mark`/`aleph_region_exit` (and an allocated-bytes
  counter for its unit tests); the generated C calls them around user
  function bodies.
- ADR-0007's "copied (or promoted)" becomes "reset, compacted or promoted to
  the caller" as above.
