#!/usr/bin/env python3
"""Pilot sessions: create, check and summarise. Python 3 standard library only.

Usage:
  pilot.py new SLUG [--date YYYY-MM-DD]   create sessions/DATE-SLUG from the template
  pilot.py check                          validate every session, exit 1 on problems
  pilot.py report                         one line per session and totals by condition
"""
import datetime
import json
import pathlib
import re
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SESSIONS = ROOT / "sessions"
TEMPLATE = SESSIONS / "_template"

CONDITIONS = {"aleph-llms", "aleph-llms-examples", "aleph-none", "python", "bash"}
CATEGORIES = {"text", "files", "data-json", "algorithm", "other"}
VERIFIED = {"tests", "examples", "manual"}
FINAL = {"python": "final.py", "bash": "final.sh"}
REQUIRED = ["id", "date", "model", "tool", "condition", "task", "task_category",
            "compiler_commit", "attempts", "passed", "verified_by",
            "reasoning_visible", "translated", "tokens", "notes"]

FENCE = re.compile(r"^```\s*([A-Za-z0-9_+-]*)\s*$")
PYTHON_LINE = re.compile(r"^\s*(def \w+\(|import \w+|from \w+ import|for \w+ in .+:|while .+:|print\()")


def sessions():
    return sorted(p for p in SESSIONS.iterdir() if p.is_dir() and not p.name.startswith("_"))


def new(slug, date=None):
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", slug):
        raise SystemExit("slug: lowercase letters, digits and dashes")
    date = date or datetime.date.today().isoformat()
    target = SESSIONS / f"{date}-{slug}"
    if target.exists():
        raise SystemExit(f"{target} already exists")
    shutil.copytree(TEMPLATE, target)
    meta = json.loads((target / "meta.json").read_text(encoding="utf-8"))
    meta["id"], meta["date"] = target.name, date
    (target / "meta.json").write_text(json.dumps(meta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(target)


def problems(folder):
    found = []
    try:
        meta = json.loads((folder / "meta.json").read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return [f"meta.json: {error}"]
    for key in REQUIRED:
        if key not in meta:
            found.append(f"missing field {key}")
    if meta.get("id") != folder.name:
        found.append("id differs from the folder name")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", str(meta.get("date", ""))):
        found.append("date is not YYYY-MM-DD")
    for key in ("model", "tool", "task", "compiler_commit"):
        if not meta.get(key):
            found.append(f"{key} is empty")
    if meta.get("condition") not in CONDITIONS:
        found.append(f"condition must be one of {sorted(CONDITIONS)}")
    if meta.get("task_category") not in CATEGORIES:
        found.append(f"task_category must be one of {sorted(CATEGORIES)}")
    if meta.get("verified_by") not in VERIFIED:
        found.append(f"verified_by must be one of {sorted(VERIFIED)}")
    if not isinstance(meta.get("attempts"), int) or meta["attempts"] < 0:
        found.append("attempts must be a whole number")
    if not isinstance(meta.get("passed"), bool):
        found.append("passed must be true or false")
    if not isinstance(meta.get("reasoning_visible"), bool):
        found.append("reasoning_visible must be true or false")
    if meta.get("translated") not in (True, False, None):
        found.append("translated must be true, false or null")
    for name in ("prompt.md", "trace.md"):
        path = folder / name
        if not path.exists() or path.read_text(encoding="utf-8").strip().startswith("<!--"):
            found.append(f"{name} is missing or still the template")
    final = FINAL.get(meta.get("condition"), "final.aph")
    if not (folder / final).exists():
        found.append(f"{final} is missing")
    return found


def check():
    bad = 0
    for folder in sessions():
        for problem in problems(folder):
            print(f"{folder.name}: {problem}")
            bad += 1
    print(f"{len(sessions())} sessions, {bad} problems")
    return 1 if bad else 0


def translation_hint(trace):
    """Lines of code in another language before the first Aleph code block."""
    lines, language, hits = trace.splitlines(), None, 0
    for line in lines:
        fence = FENCE.match(line)
        if fence:
            if language is None:
                language = fence.group(1).lower() or "plain"
                if language == "aleph":
                    return hits
                if language in ("python", "py", "bash", "sh", "javascript", "js", "c"):
                    hits += 1
            else:
                language = None
            continue
        if language not in ("aleph",) and PYTHON_LINE.match(line):
            hits += 1
    return hits


def report():
    rows, totals = [], {}
    for folder in sessions():
        meta = json.loads((folder / "meta.json").read_text(encoding="utf-8"))
        trace = (folder / "trace.md").read_text(encoding="utf-8") if (folder / "trace.md").exists() else ""
        hint = translation_hint(trace) if meta.get("condition", "").startswith("aleph") else 0
        rows.append((folder.name, meta.get("condition"), meta.get("model"), meta.get("attempts"),
                     meta.get("passed"), meta.get("translated"), hint))
        bucket = totals.setdefault(meta.get("condition"), {"sessions": 0, "passed": 0, "attempts": 0})
        bucket["sessions"] += 1
        bucket["passed"] += 1 if meta.get("passed") else 0
        bucket["attempts"] += meta.get("attempts") or 0
    print("session | condition | model | attempts | passed | translated | other-language lines before Aleph")
    for row in rows:
        print(" | ".join("" if value is None else str(value) for value in row))
    print()
    for condition, bucket in sorted(totals.items()):
        mean = bucket["attempts"] / bucket["sessions"]
        print(f"{condition}: {bucket['sessions']} sessions, {bucket['passed']} passed, {mean:.1f} attempts on average")
    if not rows:
        print("no sessions yet")
    return 0


def main(argv):
    if len(argv) >= 2 and argv[0] == "new":
        date = argv[argv.index("--date") + 1] if "--date" in argv else None
        new(argv[1], date)
        return 0
    if argv == ["check"]:
        return check()
    if argv == ["report"]:
        return report()
    raise SystemExit(__doc__)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
