#!/usr/bin/env python3
"""Import, classify and count coding-agent traces for EXP-001.

Usage:
  python3 traces.py import --format opencode FILE... -o OUT.jsonl [--task-type TYPE]
  python3 traces.py import --format claude-code FILE... -o OUT.jsonl [--task-type TYPE]
  python3 traces.py import --format jsonl FILE... -o OUT.jsonl [--task-type TYPE]
  python3 traces.py report SESSIONS.jsonl [--vocab PATH] [--json] [--seed 0] [--boot 1000]

Standard library only. Nothing is downloaded: a tokenizer is read from a
local tiktoken-format vocabulary when --vocab is given, otherwise the report
uses a labelled chars/4 estimate.
"""
import argparse
import base64
import json
import math
import pathlib
import random
import re
import sys

CLASSES = ["system", "user", "reasoning", "code_written", "code_read", "tool_other", "prose"]
WRITE_TOOLS = {"write", "edit", "multiedit", "notebookedit", "apply_patch", "patch", "bash", "shell", "exec"}
CODE_KEYS = {"content", "new_string", "command", "patch", "code"}
READ_TOOLS = {"read", "cat", "view", "open"}

# Approximation of tiktoken's pre-tokenizer: Python `re` has no \p{L}, so
# letters are [^\W\d_], numbers are \d and the prefix class is [^\r\n\w].
_PRETOKEN = re.compile(
    r"""(?i:'s|'t|'re|'ve|'m|'ll|'d)"""
    r"""|[^\r\n\w]?[^\W\d_]+"""
    r"""|\d{1,3}"""
    r"""| ?[^\s\w]+[\r\n]*"""
    r"""|\s*[\r\n]+"""
    r"""|\s+(?!\S)"""
    r"""|\s+"""
)
_FENCE = re.compile(r"```(?:[^\n`]*\n)?(.*?)```", re.S)


# --- classifier -----------------------------------------------------------

def classify(span):
    """Return [(class, text), ...] for one span (the seven classes above)."""
    role = (span.get("role") or "").lower()
    kind = (span.get("kind") or "text").lower()
    text = span.get("text") or ""
    if role == "system":
        return [("system", text)] if text else []
    if kind == "reasoning":
        return [("reasoning", text)] if text else []
    if kind == "tool_call":
        return _classify_tool_call(span, text)
    if kind == "tool_result":
        tool = (span.get("tool") or "").lower()
        return [("code_read" if tool in READ_TOOLS else "tool_other", text)] if text else []
    if role == "user":
        return [("user", text)] if text else []
    if role == "assistant":
        return _split_text(text)
    return [("tool_other", text)] if text else []


def _split_text(text):
    """Assistant text -> prose pieces and fenced code -> code_written."""
    if not text:
        return []
    pieces, pos = [], 0
    for match in _FENCE.finditer(text):
        if match.start() > pos:
            pieces.append(("prose", text[pos:match.start()]))
        code = match.group(1)
        if code:
            pieces.append(("code_written", code))
        pos = match.end()
    if pos < len(text):
        pieces.append(("prose", text[pos:]))
    return pieces or [("prose", text)]


def _classify_tool_call(span, text):
    tool = (span.get("tool") or "").lower()
    try:
        args = json.loads(text)
    except (TypeError, ValueError):
        args = None
    if not isinstance(args, dict):
        return [("code_written" if tool in WRITE_TOOLS else "tool_other", text)] if text else []
    pieces, other = [], {}
    for key, value in args.items():
        if tool in WRITE_TOOLS and key in CODE_KEYS and isinstance(value, str):
            if value:
                pieces.append(("code_written", value))
        else:
            other[key] = value
    if other:
        pieces.append(("tool_other", json.dumps(other, ensure_ascii=False, sort_keys=True)))
    return pieces


# --- token counter --------------------------------------------------------

class TokenCounter:
    """Real BPE count from a tiktoken vocabulary, or a labelled chars/4 estimate."""

    def __init__(self, ranks=None, vocab_path=None):
        self.ranks = ranks
        self.vocab_path = vocab_path
        self._cache = {}

    @classmethod
    def estimate(cls):
        return cls()

    @classmethod
    def load(cls, path):
        ranks = {}
        with open(path, encoding="utf-8") as source:
            for line in source:
                line = line.strip()
                if not line:
                    continue
                token, _, rank = line.partition(" ")
                ranks[base64.b64decode(token.encode())] = int(rank)
        return cls(ranks, str(path))

    @property
    def label(self):
        if self.ranks is None:
            return "estimate (chars/4)"
        return f"bpe ({self.vocab_path})"

    def count(self, text):
        if not text:
            return 0
        if self.ranks is None:
            return (len(text) + 3) // 4
        cached = self._cache.get(text)
        if cached is None:
            cached = sum(self._bpe(piece.encode("utf-8")) for piece in _PRETOKEN.findall(text))
            self._cache[text] = cached
        return cached

    def _bpe(self, data):
        parts = [bytes([byte]) for byte in data]
        while len(parts) > 1:
            best_rank = None
            best_at = None
            for i in range(len(parts) - 1):
                rank = self.ranks.get(parts[i] + parts[i + 1])
                if rank is not None and (best_rank is None or rank < best_rank):
                    best_rank, best_at = rank, i
            if best_at is None:
                break
            parts[best_at:best_at + 2] = [parts[best_at] + parts[best_at + 1]]
        return len(parts)


# --- importers ------------------------------------------------------------

def _read_jsonl(path):
    with open(path, encoding="utf-8") as source:
        for line in source:
            line = line.strip()
            if not line:
                continue
            try:
                yield json.loads(line)
            except ValueError:
                continue


def _as_text(value):
    if isinstance(value, str):
        return value
    if value is None:
        return ""
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def _span(role, kind, text, tool=None, cached=False):
    return {"role": role, "kind": kind, "tool": tool, "text": text, "cached": bool(cached)}


def _session(session, source, task_type, spans, reasoning_visible=None):
    if reasoning_visible is None:
        reasoning_visible = any(s["kind"] == "reasoning" and bool(s["text"]) for s in spans)
    return {
        "session": str(session),
        "source": source,
        "task_type": task_type,
        "reasoning_visible": bool(reasoning_visible),
        "spans": spans,
    }


def import_opencode(path, task_type=None):
    fallback = pathlib.Path(path).name
    sessions = {}
    for event in _read_jsonl(path):
        name = event.get("sessionID") or fallback
        spans = sessions.setdefault(name, [])
        part = event.get("part") or {}
        cached = bool(event.get("cached") or part.get("cached"))
        if event.get("type") == "text":
            spans.append(_span("assistant", "text", _as_text(part.get("text")), cached=cached))
        elif event.get("type") == "reasoning":
            spans.append(_span("assistant", "reasoning", _as_text(part.get("text")), cached=cached))
        elif event.get("type") == "tool_use":
            tool = part.get("tool") or part.get("name") or ""
            state = part.get("state") or {}
            args = state.get("input") or {}
            spans.append(_span("assistant", "tool_call",
                               json.dumps(args, ensure_ascii=False, sort_keys=True), tool, cached))
            if state.get("output") is not None:
                spans.append(_span("tool", "tool_result", _as_text(state.get("output")), tool, cached))
    return [_session(name, "opencode", task_type, spans) for name, spans in sessions.items()]


def import_claude_code(path, task_type=None):
    records = list(_read_jsonl(path))
    session_id = next((r.get("sessionId") for r in records if r.get("sessionId")), pathlib.Path(path).name)
    spans, tool_names = [], {}
    for record in records:
        kind = record.get("type")
        if kind not in ("user", "assistant", "system"):
            continue
        message = record.get("message") or {}
        content = message.get("content", record.get("content"))
        for block in _blocks(content):
            if isinstance(block, str):
                role = "system" if kind == "system" else ("assistant" if kind == "assistant" else "user")
                spans.append(_span(role, "text", block))
                continue
            block_type = block.get("type")
            cached = bool(block.get("cached"))
            if kind == "system" and block_type in (None, "text"):
                spans.append(_span("system", "text", _as_text(block.get("text")), cached=cached))
            elif block_type == "text":
                role = "assistant" if kind == "assistant" else "user"
                spans.append(_span(role, "text", _as_text(block.get("text")), cached=cached))
            elif block_type == "thinking" and kind == "assistant":
                text = block.get("thinking") or block.get("text") or ""
                spans.append(_span("assistant", "reasoning", _as_text(text), cached=cached))
            elif block_type == "tool_use" and kind == "assistant":
                tool = block.get("name") or ""
                tool_names[block.get("id")] = tool
                args = block.get("input") or {}
                spans.append(_span("assistant", "tool_call",
                                   json.dumps(args, ensure_ascii=False, sort_keys=True), tool, cached))
            elif block_type == "tool_result":
                tool = tool_names.get(block.get("tool_use_id")) or block.get("tool") or ""
                spans.append(_span("tool", "tool_result", _result_text(block.get("content")),
                                   tool, cached))
    return [_session(session_id, "claude-code", task_type, spans)]


def _blocks(content):
    if content is None:
        return []
    if isinstance(content, list):
        return content
    return [content]


def _result_text(content):
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return _as_text(content)
    parts = []
    for block in content:
        if isinstance(block, str):
            parts.append(block)
        elif isinstance(block, dict) and block.get("type") == "text":
            parts.append(_as_text(block.get("text")))
    return "\n".join(parts)


def import_jsonl(path, task_type=None):
    sessions = []
    fallback = pathlib.Path(path).name
    for index, record in enumerate(_read_jsonl(path), 1):
        spans = []
        for span in record.get("spans") or []:
            spans.append(_span(span.get("role") or "assistant", span.get("kind") or "text",
                               span.get("text") or "", span.get("tool"), span.get("cached")))
        sessions.append(_session(record.get("session") or f"{fallback}:{index}",
                                 record.get("source") or "other",
                                 record.get("task_type") if task_type is None else task_type,
                                 spans, record.get("reasoning_visible")))
    return sessions


def import_files(fmt, paths, task_type=None):
    sessions = []
    for path in paths:
        if fmt == "opencode":
            sessions.extend(import_opencode(path, task_type))
        elif fmt == "claude-code":
            sessions.extend(import_claude_code(path, task_type))
        else:
            sessions.extend(import_jsonl(path, task_type))
    return sessions


# --- report ---------------------------------------------------------------

def _session_rows(sessions, counter):
    rows = []
    for session in sessions:
        counts = {name: 0 for name in CLASSES}
        cached = {name: 0 for name in CLASSES}
        for span in session.get("spans") or []:
            is_cached = bool(span.get("cached"))
            for name, text in classify(span):
                tokens = counter.count(text)
                (cached if is_cached else counts)[name] += tokens
        rows.append({"counts": counts, "cached": cached, "total": sum(counts.values())})
    return rows


def _cached_summary(rows):
    tokens = {name: sum(row["cached"][name] for row in rows) for name in CLASSES}
    return {"tokens": tokens, "total": sum(tokens.values())}


def _percentile(values, q):
    if not values:
        return 0.0
    position = (len(values) - 1) * q
    low, high = math.floor(position), math.ceil(position)
    if low == high:
        return float(values[low])
    return values[low] * (high - position) + values[high] * (position - low)


def _group_stats(rows, seed, boot):
    classes = CLASSES
    count = len(rows)
    tokens = {name: sum(row["counts"][name] for row in rows) for name in classes}
    total = sum(tokens.values())
    pooled = {name: (tokens[name] / total if total else 0.0) for name in classes}
    shares = [{name: (row["counts"][name] / row["total"] if row["total"] else 0.0)
               for name in classes} for row in rows]
    mean = {name: (sum(share[name] for share in shares) / count if count else 0.0)
            for name in classes}

    rng = random.Random(seed)
    pooled_samples = {name: [] for name in classes}
    mean_samples = {name: [] for name in classes}
    for _ in range(boot):
        sample = [rows[rng.randrange(count)] for _ in range(count)] if count else []
        sampled = {name: sum(row["counts"][name] for row in sample) for name in classes}
        sampled_total = sum(sampled.values())
        for name in classes:
            pooled_samples[name].append(sampled[name] / sampled_total if sampled_total else 0.0)
            if sample:
                mean_samples[name].append(
                    sum(row["counts"][name] / row["total"] if row["total"] else 0.0 for row in sample) / len(sample))
            else:
                mean_samples[name].append(0.0)

    return {
        "sessions": count,
        "total": total,
        "tokens": tokens,
        "pooled": {"shares": pooled, "ci": {name: _interval(pooled_samples[name]) for name in classes}},
        "mean_per_session": {"shares": mean, "ci": {name: _interval(mean_samples[name]) for name in classes}},
    }


def _interval(values):
    ordered = sorted(values)
    return [_percentile(ordered, 0.025), _percentile(ordered, 0.975)]


def _task_type(session):
    return session.get("task_type") or "unknown"


def _has_visible_reasoning(session):
    if "reasoning_visible" in session:
        return bool(session["reasoning_visible"])
    return any(span.get("kind") == "reasoning" and span.get("text") for span in session.get("spans") or [])


def report(sessions, counter=None, seed=0, boot=1000):
    """Return the report numbers as a dict (also used by --json)."""
    counter = counter or TokenCounter.estimate()
    sessions = list(sessions)
    visible = [session for session in sessions if _has_visible_reasoning(session)]
    hidden = [session for session in sessions if not _has_visible_reasoning(session)]
    visible_rows = _session_rows(visible, counter)
    hidden_rows = _session_rows(hidden, counter)

    overall = _group_stats(visible_rows, seed, boot)
    by_task = {}
    for task_type in sorted({_task_type(session) for session in visible}):
        subset = [session for session in visible if _task_type(session) == task_type]
        by_task[task_type] = _group_stats(_session_rows(subset, counter), seed, boot)

    by_tercile = {}
    ordered = sorted(visible_rows, key=lambda row: row["total"])
    buckets = {"short": [], "medium": [], "long": []}
    for index, row in enumerate(ordered):
        label = ["short", "medium", "long"][min(2, index * 3 // len(ordered))] if ordered else "short"
        buckets[label].append(row)
    for label, rows in buckets.items():
        if rows:
            by_tercile[label] = _group_stats(rows, seed, boot)

    code = overall["pooled"]["shares"]["code_written"] + overall["pooled"]["shares"]["code_read"]
    no_reasoning = _group_stats(hidden_rows, seed, boot)
    no_reasoning["cached"] = _cached_summary(hidden_rows)
    return {
        "counter": counter.label,
        "seed": seed,
        "boot": boot,
        "sessions": {"visible": len(visible), "no_reasoning": len(hidden)},
        "overall": overall,
        "by_task_type": by_task,
        "by_length_tercile": by_tercile,
        "no_reasoning": no_reasoning,
        "cached": _cached_summary(visible_rows),
        "s_max": {str(r): code * (1 - r) for r in (0.5, 0.7, 0.9)},
    }


def format_report(result):
    lines = [f"token counter: {result['counter']}"]
    if result["counter"] == "estimate (chars/4)":
        lines.append("note: counts are estimate (chars/4), not real tokenizer counts")
    sessions = result["sessions"]
    lines.append(f"sessions: {sessions['visible']} with visible reasoning, "
                 f"{sessions['no_reasoning']} without")
    lines.extend(_format_group("overall (visible reasoning, pooled)", result["overall"]))
    for task_type, group in result["by_task_type"].items():
        lines.extend(_format_group(f"task_type: {task_type}", group))
    for label, group in result["by_length_tercile"].items():
        lines.extend(_format_group(f"length tercile: {label}", group))
    cached = result["cached"]
    lines.append(f"cached tokens (excluded from shares): {cached['total']}")
    lines.extend(_format_group("without visible reasoning (separate, never pooled)",
                               result["no_reasoning"]))
    lines.append(f"cached tokens in those sessions: {result['no_reasoning']['cached']['total']}")
    for r, value in result["s_max"].items():
        lines.append(f"S_max r={r}: {value:.3f}")
    return "\n".join(lines)


def _format_group(title, group):
    lines = ["", title, f"  sessions: {group['sessions']}  counted tokens: {group['total']}",
             f"  {'class':<12}{'pooled':>9}{'95% CI':>19}{'mean/session':>15}{'95% CI':>19}"]
    for name in CLASSES:
        pooled = group["pooled"]["shares"][name]
        mean = group["mean_per_session"]["shares"][name]
        lines.append(f"  {name:<12}{pooled:>9.3f}{_ci(group['pooled']['ci'][name]):>19}"
                     f"{mean:>15.3f}{_ci(group['mean_per_session']['ci'][name]):>19}")
    return lines


def _ci(interval):
    return f"[{interval[0]:.3f}, {interval[1]:.3f}]"


# --- command line ---------------------------------------------------------

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)

    importer = commands.add_parser("import", help="convert traces to the JSONL schema")
    importer.add_argument("--format", choices=["opencode", "claude-code", "jsonl"], required=True)
    importer.add_argument("files", nargs="+")
    importer.add_argument("-o", "--out", required=True)
    importer.add_argument("--task-type", default=None)

    reporter = commands.add_parser("report", help="token shares, intervals and S_max")
    reporter.add_argument("sessions")
    reporter.add_argument("--vocab", help="local tiktoken-format vocabulary file")
    reporter.add_argument("--json", action="store_true")
    reporter.add_argument("--seed", type=int, default=0)
    reporter.add_argument("--boot", type=int, default=1000)

    args = parser.parse_args(argv)
    if args.command == "import":
        sessions = import_files(args.format, args.files, args.task_type)
        with open(args.out, "w", encoding="utf-8") as out:
            for session in sessions:
                out.write(json.dumps(session, ensure_ascii=False, sort_keys=True) + "\n")
        print(f"wrote {len(sessions)} session(s) to {args.out}", file=sys.stderr)
    else:
        counter = TokenCounter.load(args.vocab) if args.vocab else TokenCounter.estimate()
        result = report(_read_jsonl(args.sessions), counter, args.seed, args.boot)
        if args.json:
            print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
        else:
            print(format_report(result))


if __name__ == "__main__":
    main()
