# ADR-0039: Scoped regions as mark/reset with result compaction

Status: Accepted
Accepted: 2026-11-27, by the owner (v0.2-experiment freeze)
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
  `@f` values) and every lambda call (one region in `aleph_lam_n<id>`
  around both bodies, T-053) opens a region; `main` and builtins do not.
  Lambdas are safe for the same reason as `@f` callbacks: binders and
  captured values (copied when the `lam` node is evaluated) are older than
  the mark, and `map`/`filter`/`fold`/`map2` store a result, or pass a
  `fold` accumulator on, only after the call has returned. So a
  `fold lam ...` in `main` reclaims each step's temporaries.
- Mark: the arena top (current chunk and its `used`) plus the live bytes
  held by older chunks, so the region size is a subtraction. The root check
  returns at once when the root lies in the newest chunk (the usual case);
  otherwise it walks the region's chunks, so an older root under a region of
  many chunks costs one walk per return (call depth x chunks at worst).
  Compaction builds a sorted array of the region's chunk ranges once and
  binary-searches it.
- A function or lambda body that cannot allocate (only unboxed literals,
  variables, operators, `if` and user calls, every node unboxed) gets no
  region: what it leaves is reclaimed by the caller's region. This keeps
  `fib`-like code as fast as before.
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
     the object that owns them. The new chunk has exactly the copied size
     (a 64 KiB one would stay alive, mostly empty, under the caller's next
     marks: 64 KiB per call in a loop) and becomes the newest one; the next
     allocation takes a new or the spare chunk. If an internal `malloc`
     fails, compaction gives up and falls back to rule 3: it never adds a
     trap.
  3. Otherwise leave the region as it is: the caller's region absorbs it.
     The common case (root in the newest chunk past the mark and a region
     below 256 KiB) is decided inline in `aleph_region_exit`, without
     calling `aleph_region_leave`.
- Scan budget: a failed compaction attempt (rule 2 traversal past its
  limit) adds the region's quarter to a global count of scanned bytes, and
  rule 2 is attempted only while that count is within a quarter of all
  bytes ever allocated (64-bit counters). One attempt can overshoot by its
  own quarter, so failed scans stay under about half of the bytes
  allocated. Checking the attempt's quarter before trying instead skipped
  compactions that would succeed in deep builders (`run_region_grow_list`
  kept 45 MB instead of 0.6 MB). One large failed attempt can pause
  compaction until about as much again has been allocated. Without the
  budget, data a deep recursion leaves to its callers is rescanned by every
  outer level (quadratic in depth: a 4500-deep builder took 14 s natively
  instead of 0.03 s); with it, scanning stays linear in allocation.
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
- Rule 2 only applies to regions of at least 256 KiB. A call that leaves
  less garbage than that plus a small boxed result promotes its whole
  region to the caller: bounded per call, but a loop in `main` (no region)
  over such calls still grows with the iteration count.
- A tail of costs: three loads at the mark and a compare per call on the
  common path; compaction costs at most a quarter of the bytes the region
  allocated, failed attempts about half of all bytes allocated.
- Tlön gains `aleph_region_mark`/`aleph_region_exit` (and arena-bytes and
  scanned-bytes counters for its unit tests); the generated C calls them
  around user function bodies.
- ADR-0007's "copied (or promoted)" becomes "reset, compacted or promoted to
  the caller" as above.
