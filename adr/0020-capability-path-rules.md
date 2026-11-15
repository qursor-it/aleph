# ADR-0020: Capability path rules and C003

Status: Proposed
Date: 2026-11-15

## Context
SPEC §3.1 and ADR-0008 require filesystem capabilities to be declared as path
prefixes, compared after normalization, with `..` forbidden in prefixes. The
spec leaves the exact normalization and the diagnostic for a malformed `use`
line open. The interpreter needs the same rule for its runtime check (R003).

## Decision
- Paths are POSIX paths only. A path or prefix is normalized by splitting on
  `/` and dropping empty and `.` segments; a leading `//` is therefore treated
  as `/`. A path or prefix containing a `..` segment is never covered.
- `caps::covers(prefix, path)` is true when both are relative or both are
  absolute and the normalized prefix segments are a leading run of the
  normalized path segments. Thus `data` covers `data` and `data/in.txt` but
  not `database/x`. The relative prefix `""` or `"."` covers every relative
  path; an absolute path is covered only by an absolute prefix (`/` covers all
  absolute paths).
- `covers` is a lexical early filter for C002 only. The interpreter's runtime
  check (R003, T-013) MUST resolve symlinks before comparing: `resolve_lenient`
  rejects `..`, canonicalizes the longest existing ancestor (or the whole path
  when it exists, including symlinks) and appends any missing components, then
  compares the resolved absolute paths component-wise with
  `Path::starts_with` (ADR-0021). Compile-time C002 can therefore accept a path
  that R003 rejects after symlink resolution.
- An empty prefix, a prefix with a `..` segment, or a filesystem capability
  without a path is C003 "invalid capability" at the `use` node. The whole
  working directory must be declared explicitly as `"."`. A bad prefix uses
  the message "invalid capability prefix" with `found` set to the prefix; a
  missing path uses "missing capability path" with `found` set to `""`.
  `use net.host`, `use env` and `use time` are accepted as declared
  capabilities in v0.1 even without a path.
- C001 is reported when a call needs a capability no `use` declares; C002 is
  reported only when the capability is declared but no declared prefix covers
  a literal path argument. A missing capability therefore does not also
  produce C002.

## Consequences
- C002 and R003 share the same lexical prefix notion, but R003 resolves both
  sides to absolute canonical paths before `Path::starts_with`; the two checks
  can therefore differ only where symlinks change the resolved path.
- `aleph explain` lists capabilities via `caps::capabilities`, sorted and
  deduplicated.
