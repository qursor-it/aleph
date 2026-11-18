# Pilot sessions

Informal sessions run by hand while the protocols are in draft: an agent writing Aleph (or Python, for comparison) on small tasks. They are **exploratory**: they help to design EXP-001 and EXP-002, and they never count as evidence for or against a hypothesis, because they were not planned in advance.

Only people add sessions here. The loop agent does not write in this folder.

## One folder per session

```
sessions/2026-11-18-gcd-recursive/
  meta.json     what was run, by whom, with which model, and how it went
  prompt.md     the exact prompt given to the agent, including any spec or examples pasted in
  trace.md      the full conversation as exported, reasoning included when the tool shows it
  final.aph     the final program (final.py or final.sh for the other conditions)
```

Create one from the template:

```sh
python3 experiments/pilot/tools/pilot.py new gcd-recursive
```

Then fill in `meta.json`, paste the prompt and the trace, save the final program.

## meta.json

| Field | Meaning |
|---|---|
| `id` | the folder name |
| `date` | `YYYY-MM-DD` |
| `model` | exact model name and version, as the tool reports it |
| `tool` | where it ran: chat app, opencode, MCP client, script |
| `condition` | `aleph-llms` (llms.txt only), `aleph-llms-examples` (llms.txt plus solved examples), `aleph-none` (no spec), `python`, `bash` |
| `task` | one line: what the program must do |
| `task_category` | `text`, `files`, `data-json`, `algorithm`, `other` |
| `compiler_commit` | `git rev-parse --short HEAD` of the compiler used |
| `attempts` | how many times the agent ran `aleph check` (or the program / tests) before it worked or gave up |
| `passed` | `true` if the final program does what the task asks |
| `verified_by` | how you know: `tests`, `examples`, `manual` |
| `reasoning_visible` | `true` if the trace shows the model's reasoning |
| `translated` | your judgement after reading the trace: `true` if it drafted the solution in another language first, `false` if not, `null` if you cannot tell |
| `tokens` | `input`, `cached_input`, `output`, `reasoning` when the tool reports them, otherwise `null` |
| `notes` | anything surprising: wrong builtins, misread syntax, useful or useless diagnostics |

## Checking and summarising

```sh
python3 experiments/pilot/tools/pilot.py check    # every session has the required files and fields
python3 experiments/pilot/tools/pilot.py report   # one line per session and totals by condition
```

`report` also flags traces that contain code in another language before the first Aleph code, as a hint for the `translated` judgement. The hint is a heuristic: the judgement stays yours.
