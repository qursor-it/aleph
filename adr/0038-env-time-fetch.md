# ADR-0038: Builtins `env`, `now`, `fetch`

Status: Proposed
Date: 2026-11-19

## Context
T-031. SPEC §3.1 and ADR-0008 already parse `use net.host "HOST"`, `use env`
and `use time`, but §5 says only "`fetch`, `now`, `env`: v0.2", so the three
capabilities grant nothing. Agent tasks need to read configuration,
timestamps and HTTP APIs. Constraints: no new crates (AGENTS.md), both
engines byte-identical where the world allows it (T-020), deny by default
with static checks for literals (§8), hermetic examples (ADR-0022), a full
`llms.txt` (ADR-0026).

The 18 Nov pilot (experiments/pilot/probe) found the token cost in rules and
missing builtins, not in capabilities, so this ADR adds three builtins and no
new rules; the paused T-031 was resumed by the project owner on 19 Nov 2026.

Options for `fetch` transport (score 1-10):
- Run the system `curl` binary (argv, no shell) in both engines: HTTPS, proxies
  and timeouts for free, identical behaviour in the interpreter and Tlön, no
  library linked. Needs `curl` installed at run time. **8**
- Hand-written HTTP/1.1 over sockets in Rust and in C: no external program,
  but no HTTPS without a TLS crate/library, which is most real APIs. **4**
- Link libcurl into Tlön and add an HTTP crate to the interpreter: a new crate
  (needs its own ADR) and a build dependency for every user. **3**

## Decision
- `env name` : `str -> opt str`, capability `use env`. `some` of the variable's
  value; `none` if it is unset, its value is not valid UTF-8, or `name` is
  empty or contains `=` or NUL.
- `now` : `i64` (no arguments, like `args`), capability `use time`.
  Milliseconds since the Unix epoch, wall clock (negative before 1970).
- `fetch url` : `str -> res str str`, capability `use net.host "HOST"`. HTTP
  GET of `url`; `ok` of the response body.
  - `url` starts with `http://` or `https://`; its host is the text after
    `//` up to the first `/`, `?`, `#` or `:`. A URL with another scheme, an
    empty host or a `@` before the first `/` has no host.
  - Covered iff its host equals a declared `net.host` value, ASCII case
    ignored; the port is not part of the match. A literal `url` is checked
    statically (C001 without any `net.host`, C002 when not covered or without
    a host, with a certain fix `use net.host "HOST"` as for paths, ADR-0028);
    a computed one at run time (R003).
  - A `net.host` value must be a bare host name or IPv4 address,
    `[A-Za-z0-9.-]+`; anything else (empty, a port, a scheme, `%`, brackets)
    is C003. IPv6 literals cannot be declared yet.
  - Transport: `curl -q -g -sS --fail --max-time 30 --max-redirs 0
    --proto =http,https --max-filesize 16777216 -- URL`, run without a shell
    (`-g`: no URL globbing). Redirects are not followed: a 3xx response is
    `ok` of its body. Reading stops after 16 MiB + 1 bytes and curl is
    killed. curl honours the usual proxy and CA environment variables; the
    host check applies to the URL, so they cannot widen the grant.
  - `err` messages: `fetch: curl not found`; `fetch: curl failed` (curl
    could not be started, or ended without an error text, e.g. killed);
    `fetch: response too large` above 16 MiB (also curl exit 63); `fetch:
    body is not valid UTF-8`; otherwise `fetch: ` followed by curl's error
    text with its `curl: (N) ` prefix removed, passed through as is (e.g.
    `fetch: The requested URL returned error: 404`; a timeout names the
    elapsed milliseconds). Failures to set up the child process (pipe, fork)
    are `fetch: curl failed`. Tlön runs curl with `execvp`, which also runs
    an executable script without a `#!` line through `/bin/sh`; the
    interpreter does not, so only a broken `curl` on PATH can make the
    engines' error texts differ.
- All three trap with R003 inside examples (hermetic, as `read`/`write`).
- `env`, `now` and `fetch` become reserved names (T005).

## Consequences
- `env`/`now`/`fetch` results depend on the world: the differential tests
  compare them only through deterministic properties (unset variable,
  `now > 0`, a loopback HTTP server in the integration tests).
- `fetch` needs `curl` on the machine that runs the program; without it every
  call is `err`, never a trap.
- `use env` grants every variable; per-variable grants can come later.
- The allowlist admits numeric host forms curl accepts (`0`, `2130706433`
  reach 127.0.0.1); they still need an explicit declaration, which is what a
  reviewer reads. A URL without a port uses the scheme's default port.
- `llms.txt` replaces "no builtins yet" with three builtin lines (ADR-0026
  budget).

## Amendments (review round 1, 19 Nov 2026)
- `net.host` values became an allowlist (`[A-Za-z0-9.-]+`): with the first
  blocklist, `use net.host "["` granted every IPv6 literal and `%31` hosts
  read differently from what curl contacted.
- Added `-g` (curl globbing sent several requests for `[1-3]`), the streaming
  16 MiB cut-off (curl 7.81 enforces `--max-filesize` only with a
  Content-Length), the exit-63 and `curl failed` messages, and `now` before
  1970.
- Round 2: native stderr moved from a /tmp file to a pipe read with poll
  (no temp file, no TMPDIR dependence), child stdin from /dev/null, set-up
  failures are `curl failed`, `env` names with `=` or empty are `none`.
- Round 3: the certain fix never offers a host outside the allowlist (an
  IPv6 or `_` URL gets no fix); curl error text that is not UTF-8 is `curl
  failed` in both engines; a failed stdout read is `fetch: io error` in both;
  curl gets the default SIGPIPE. curl's stderr is not capped: only a hostile
  `curl` on PATH could make it large.
