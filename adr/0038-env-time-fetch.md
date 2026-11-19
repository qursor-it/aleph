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
  value; `none` if it is unset or its value is not valid UTF-8.
- `now` : `i64` (no arguments, like `args`), capability `use time`.
  Milliseconds since the Unix epoch, wall clock.
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
  - A `net.host` value that is empty or contains `/`, `:`, `@`, `?`, `#` or
    whitespace is C003.
  - Transport: `curl -q -sS --fail --max-time 30 --max-redirs 0
    --proto =http,https --max-filesize 16777216 -- URL`, run without a shell.
    Redirects are not followed.
  - `err` messages: `fetch: curl not found`; `fetch: ` followed by curl's
    error text with its `curl: (N) ` prefix removed (e.g. `fetch: The
    requested URL returned error: 404 Not Found`); `fetch: response too
    large` above 16 MiB; `fetch: body is not valid UTF-8`.
- All three trap with R003 inside examples (hermetic, as `read`/`write`).
- `env`, `now` and `fetch` become reserved names (T005).

## Consequences
- `env`/`now`/`fetch` results depend on the world: the differential tests
  compare them only through deterministic properties (unset variable,
  `now > 0`, a loopback HTTP server in the integration tests).
- `fetch` needs `curl` on the machine that runs the program; without it every
  call is `err`, never a trap.
- `use env` grants every variable; per-variable grants can come later.
- `llms.txt` replaces "no builtins yet" with three builtin lines (ADR-0026
  budget).
