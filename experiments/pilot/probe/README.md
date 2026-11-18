# Probe: direct or translated?

A quick, informal check of one question before the experiments run: when a model writes Aleph, does it reason in Aleph, or draft the solution in Python first and translate it? Ten HumanEval-Aleph problems, chosen to be small and varied.

| File | |
|---|---|
| `KIT.md` | the ten prompts, to paste by hand into a chat with a model that shows its reasoning |
| `probe.json` | the same prompts, with `llms.txt` as system message, for the script |
| `probe.py` | runs them on a local Ollama model or an OpenAI-compatible API (DeepSeek), saves reasoning, answer and tokens in `probe-results.jsonl`; standard library only, hard cap on calls |
| `analyze.py` | scores each answer with the hidden tests and counts signs of another language in the reasoning |

```sh
python3 probe.py --ollama --model qwen3:8b
python3 probe.py --base-url https://api.deepseek.com --model deepseek-reasoner --max-calls 10   # needs DEEPSEEK_API_KEY
ALEPH=path/to/aleph python3 analyze.py probe-results.jsonl
```

This is a pilot: ten problems, no repetitions, a heuristic for translation. It can show that translation happens; it cannot prove that it does not, and it is not evidence for EXP-002.
