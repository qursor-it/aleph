# ADR-0030: Hand-written MCP server over stdio (`aleph mcp`)

Status: Proposed
Date: 2026-11-16

## Context
T-025 asks for an MCP server exposing spec, check, run, test, fmt, explain and
edit (design doc §5.5), and for a Proposed ADR on the crate choice first.
AGENTS.md allows only `serde`/`serde_json`; any other crate needs approval.

Options (score 1-10 for this repo):
- Hand-written JSON-RPC 2.0 over stdio with `serde_json`: no new dependency,
  small surface (initialize, tools, resources), fully testable. **9**
- An MCP SDK crate (e.g. `rmcp`): complete protocol, but pulls an async
  runtime and many transitive crates, against the dependency rule. **3**

## Decision
- `aleph mcp` (no FILE) serves JSON-RPC 2.0 on stdin/stdout, one JSON message
  per line, until end of input. Batches are not supported (-32600). A line
  that is not UTF-8 gets -32700 and the session continues. A request whose
  `jsonrpc` is not `"2.0"` or whose `id` is neither a string nor a number
  (MCP forbids null) gets -32600.
- Methods: `initialize` (always answers `protocolVersion` `2025-06-18`, the
  only revision served, so the client decides whether to continue;
  capabilities `tools`, `resources`), `ping`, `shutdown`
  (empty result), `tools/list`, `tools/call`, `resources/list`,
  `resources/read`. Messages without `id` are notifications and get no reply.
  Errors: -32700 parse, -32600 invalid request, -32601 unknown method,
  -32602 invalid params, -32603 internal, -32002 unknown resource.
- Tools: `spec` (`detail` `compact` = llms.txt, `full` = SPEC.md), `check`,
  `test`, `fmt`, `ast`, `explain` (`json`), `edit` (`edits`, ADR-0029), `run`
  (`args`). Source tools take inline `source`, optional `file` (a plain
  `NAME.aph`, default `main.aph`, used in diagnostics) and `timeout_ms`
  (default 5000, max 60000). `build` is omitted: it needs a C compiler and
  produces a binary, which does not fit a text tool result.
- Each source tool writes `source` into a fresh temp directory and runs the
  same `aleph` binary (`current_exe`) there with an empty environment, null
  stdin and a timeout (killed when exceeded). The temp directory is private
  (mode 0700). On Linux the child gets `PR_SET_PDEATHSIG` = SIGKILL (it dies
  with the server) and a 1 GiB address-space limit (`RLIMIT_AS`; the
  interpreter reserves a 256 MiB stack). Tool arguments longer than 100 KiB
  (`edits`, `run` args) are rejected with -32602, below Linux's 128 KiB
  argument limit. Stdout and stderr are each capped at 1 MiB. The text result is a JSON object
  `{exit_code, timed_out, truncated, stdout, stderr}`; `isError` is true when
  the exit code is not 0 or the run timed out.
- `run` additionally rejects programs whose `use` lines declare an absolute
  prefix, so the program can only touch its empty temp directory (relative
  prefixes never cover absolute paths, ADR-0020).
- Resources: `aleph://llms.txt`, `aleph://spec/SPEC.md`,
  `aleph://spec/grammar.ebnf`, `aleph://spec/diagnostics.md`.

## Consequences
- No new dependency; the protocol subset is small enough to keep correct by
  tests (`compiler/tests/mcp.rs`).
- Tools reuse the CLI exactly, so their output equals `aleph <command>`.
- The sandbox is process-level, not an OS jail: no disk quota for `fs.write`
  inside the temp dir, no CPU limit beyond the timeout, and a temp dir is left
  behind if the server is killed mid-call. Adequate for a local agent, not
  for untrusted remote clients.
- The child limits use two `libc` calls declared by hand in one small
  `unsafe` block (no `libc` crate); other platforms get no limits.
