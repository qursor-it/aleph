#!/usr/bin/env python3
"""Run the probe problems on a model and save reasoning and answers.

Usage (run it on your own computer, Python 3.10+, standard library only):

  # local model with Ollama (free); the model must support thinking, e.g. qwen3 or deepseek-r1
  python3 probe.py --ollama --model qwen3:8b

  # DeepSeek API (reads the key from the DEEPSEEK_API_KEY environment variable)
  python3 probe.py --base-url https://api.deepseek.com --model deepseek-reasoner --max-calls 10

Writes probe-results.jsonl next to this script: one line per problem with the
model, the reasoning, the answer, the code block and token counts when the
server reports them. Send that file back: evaluation runs where the Aleph
compiler is.
"""
import argparse
import json
import os
import pathlib
import re
import sys
import time
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent


def post(url, body, headers, timeout=600):
    request = urllib.request.Request(url, data=json.dumps(body).encode(), headers=headers, method="POST")
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode())


def ask_ollama(args, system, user):
    body = {"model": args.model, "stream": False, "think": True,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}]}
    data = post(args.ollama_url.rstrip("/") + "/api/chat", body, {"Content-Type": "application/json"})
    message = data.get("message", {})
    usage = {"input": data.get("prompt_eval_count"), "output": data.get("eval_count")}
    return message.get("thinking") or "", message.get("content") or "", usage


def ask_openai(args, system, user):
    key = os.environ.get(args.key_env)
    if not key:
        raise SystemExit(f"set {args.key_env} first")
    body = {"model": args.model,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}]}
    headers = {"Content-Type": "application/json", "Authorization": f"Bearer {key}"}
    data = post(args.base_url.rstrip("/") + "/chat/completions", body, headers)
    message = data["choices"][0]["message"]
    reasoning = message.get("reasoning_content") or message.get("reasoning") or ""
    usage = data.get("usage", {})
    details = usage.get("completion_tokens_details") or {}
    return reasoning, message.get("content") or "", {
        "input": usage.get("prompt_tokens"),
        "cached_input": usage.get("prompt_cache_hit_tokens"),
        "output": usage.get("completion_tokens"),
        "reasoning": details.get("reasoning_tokens"),
    }


def code_block(answer):
    blocks = re.findall(r"```[a-zA-Z]*\n(.*?)```", answer, re.S)
    return blocks[-1] if blocks else answer


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--model", required=True)
    parser.add_argument("--ollama", action="store_true", help="use a local Ollama server")
    parser.add_argument("--ollama-url", default="http://localhost:11434")
    parser.add_argument("--base-url", help="OpenAI-compatible endpoint, e.g. https://api.deepseek.com")
    parser.add_argument("--key-env", default="DEEPSEEK_API_KEY")
    parser.add_argument("--max-calls", type=int, default=10, help="hard cap on API calls")
    parser.add_argument("--out", default=str(HERE / "probe-results.jsonl"))
    args = parser.parse_args()
    if not args.ollama and not args.base_url:
        raise SystemExit("choose --ollama or --base-url")
    probe = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))
    ask = ask_ollama if args.ollama else ask_openai
    out = open(args.out, "a", encoding="utf-8")
    for number, problem in enumerate(probe["problems"][: args.max_calls], 1):
        user = probe["instruction"] + "\n\n```aleph\n" + problem["prompt"] + "```"
        started = time.time()
        try:
            reasoning, answer, usage = ask(args, probe["system"], user)
        except Exception as error:  # keep going, record the failure
            print(f"{number:02d} {problem['name']}: {error}", file=sys.stderr)
            continue
        record = {"name": problem["name"], "model": args.model, "server": "ollama" if args.ollama else args.base_url,
                  "date": time.strftime("%Y-%m-%d"), "seconds": round(time.time() - started, 1),
                  "reasoning": reasoning, "answer": answer, "code": code_block(answer), "tokens": usage}
        out.write(json.dumps(record, ensure_ascii=False) + "\n")
        out.flush()
        print(f"{number:02d} {problem['name']}: {len(reasoning)} chars of reasoning, {round(time.time() - started)} s")
    print(f"saved to {args.out}")


if __name__ == "__main__":
    main()
