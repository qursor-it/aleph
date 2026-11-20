# EXP-001 trace tools

Import coding-agent traces, classify every span, count tokens and report the
split that bounds any Aleph token saving. Python 3, standard library only, no
network.

## Usage

```sh
# Convert opencode JSON events (or claude-code session JSONL) to one JSONL schema.
python3 traces.py import --format opencode sessions.jsonl -o imported.jsonl --task-type script
python3 traces.py import --format claude-code ~/.claude/projects/x/session.jsonl -o imported.jsonl

# Report. Without --vocab callers must treat the counts as an estimate.
python3 traces.py report imported.jsonl
python3 traces.py report imported.jsonl --vocab /path/to/o200k_base.tiktoken --json --seed 0 --boot 1000

# Tests (offline, fixtures only).
python3 -m unittest -v test_traces
```

`--format jsonl` accepts the schema below, so an adapter for another source
only has to emit it.

## Schema

One session per line:

```json
{"session": "id", "source": "opencode", "task_type": "script",
 "reasoning_visible": true,
 "spans": [{"role": "assistant", "kind": "text", "tool": null,
            "text": "...", "cached": false}]}
```

- `source`: `opencode`, `claude-code` or another source name.
- `task_type`: string or `null`; use the same labels for comparable groups.
- `reasoning_visible`: whether the trace contains visible reasoning.
- `role`: `system`, `user`, `assistant` or `tool`.
- `kind`: `text`, `reasoning`, `tool_call` or `tool_result`.
- `tool`: tool name for calls/results, else `null`.
- `text`: the span text. For `tool_call` this is a JSON object of arguments.
- `cached`: true when the trace says the tokens came from a prompt cache;
  such spans are counted in a separate block and not in the class shares.

Importers: opencode groups events by `sessionID` (file name as fallback) and
reads `text`, `reasoning` and `tool_use` events (`part.state.input` and
`part.state.output`). Claude Code reads `user`/`assistant`/`system` records
with `message.content` blocks `text`, `thinking`, `tool_use`, `tool_result`;
the result's tool name comes from the matching `tool_use` id.

## Classifier

`classify(span)` returns `(class, text)` pieces for exactly these classes:

| Class | Rule |
|---|---|
| `system` | role `system` |
| `user` | user text |
| `reasoning` | `kind` = `reasoning` |
| `prose` | assistant text outside fenced code blocks |
| `code_written` | fenced code in assistant text; code arguments (`content`, `new_string`, `command`, `patch`, `code`) of write/run tools: `write`, `edit`, `multiedit`, `notebookedit`, `apply_patch`, `patch`, `bash`, `shell`, `exec` |
| `tool_other` | other arguments of those tools, all arguments of other tools, non-read tool results |
| `code_read` | results of read-like tools: `read`, `cat`, `view`, `open` |

Tool names are matched case-insensitively. Tool arguments that are not the
listed code keys are serialized as JSON.

## Token counting

A real count needs `--vocab PATH`: a local tiktoken-format file
(`base64token rank` per line), e.g. `o200k_base.tiktoken`. The counter applies
byte-pair merges by rank to the UTF-8 bytes of each pre-token. The
pre-tokenizer is a Python `re` approximation of tiktoken's pattern (Python has
no `\p{L}`), so counts can differ slightly from tiktoken.

Without `--vocab` the report uses `ceil(chars / 4)` and labels every count
`estimate (chars/4)`; never present that as a real token count.

The report gives pooled shares and mean per-session shares with 95% percentile
bootstrap intervals over sessions (deterministic for a given `--seed`). The
same numbers are grouped by `task_type` and by session-length tercile.
Sessions with `reasoning_visible` false are reported in their own block and
never pooled with the others. `S_max = (c_w + c_r) * (1 - r)` is reported for
r = 0.5, 0.7 and 0.9.

## Collecting traces

People, not this tool, collect traces and remove secrets (API keys, tokens,
private paths) before importing. The classifier is checked by hand on 20
random sessions: if agreement is below 95% of tokens, fix the classifier
before reporting.
