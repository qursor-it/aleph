#!/usr/bin/env python3
"""Replay recorded model answers through the current compiler (T-054).

Runs `aleph check --json` on each answer raw and after `aleph fmt`; reports
check-clean rates and diagnostic codes per run file and condition. Offline,
standard library only; it never executes tests or reads them.
"""
import argparse
import collections
import glob
import json
import os
import subprocess
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))


def aleph_bin():
    return os.environ.get("ALEPH") or os.path.join(ROOT, "target", "debug", "aleph")


def run_check(binary, source, workdir):
    """Return the diagnostics (list of dicts) of `check --json` on source."""
    path = os.path.join(workdir, "solution.aph")
    with open(path, "w", encoding="utf-8") as f:
        f.write(source)
    p = subprocess.run([binary, "check", "--json", path], capture_output=True, text=True, timeout=60)
    diags = []
    for line in p.stdout.splitlines():
        try:
            d = json.loads(line)
        except ValueError:
            continue
        if isinstance(d, dict):
            diags.append(d)
    if p.returncode != 0 and not any(d.get("severity") == "error" for d in diags):
        diags.append({"code": "CRASH", "severity": "error", "msg": "check failed without diagnostics"})
    return diags


def run_fmt(binary, source, workdir):
    """Return the canonical text, or None if fmt fails."""
    path = os.path.join(workdir, "solution.aph")
    with open(path, "w", encoding="utf-8") as f:
        f.write(source)
    p = subprocess.run([binary, "fmt", path], capture_output=True, text=True, timeout=60)
    return p.stdout if p.returncode == 0 else None


def errors(diags):
    return [d for d in diags if d.get("severity") == "error"]


def replay_answer(binary, code, workdir):
    raw = errors(run_check(binary, code, workdir))
    fmt = run_fmt(binary, code, workdir)
    # shortcut: if fmt fails (does not parse), the raw errors stand in for the fmt ones
    after = errors(run_check(binary, fmt, workdir)) if fmt is not None else raw
    return {
        "clean_raw": not raw,
        "clean_fmt": not after,
        "codes_raw": [d.get("code") for d in raw],
        "codes_fmt": [d.get("code") for d in after],
        "first_code": after[0].get("code") if after else None,
        "first_msg": after[0].get("msg") if after else None,
    }


def load_records(path):
    """Yield (name, condition, code) for the Aleph answers of one jsonl file."""
    with open(path, encoding="utf-8") as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            if r.get("event", "round") != "round":
                continue
            cond = str(r.get("condition") or r.get("language") or "aleph")
            if r.get("language") == "python" or not cond.startswith(("aleph", "lite", "fewshot")):
                continue
            code = r["program"] if "program" in r else r.get("code")
            yield str(r.get("name") or r.get("task") or "?"), cond, code or ""


def default_inputs():
    return sorted(glob.glob(os.path.join(ROOT, "experiments", "pilot", "probe", "runs", "*.jsonl")))


def pct(n, d):
    return round(100.0 * n / d, 1) if d else 0.0


def build_report(files, binary):
    groups = collections.OrderedDict()
    with tempfile.TemporaryDirectory() as tmp:
        for path in files:
            for name, cond, code in load_records(path):
                g = groups.setdefault((os.path.basename(path), cond), [])
                res = {"name": name, "no_code": not code.strip()}
                if not res["no_code"]:
                    res.update(replay_answer(binary, code, tmp))
                g.append(res)
    out = []
    for (fname, cond), rs in groups.items():
        coded = [r for r in rs if not r["no_code"]]
        raw_c, fmt_c = collections.Counter(), collections.Counter()
        for r in coded:
            raw_c.update(r["codes_raw"])
            fmt_c.update(r["codes_fmt"])
        out.append({
            "file": fname,
            "condition": cond,
            "answers": len(rs),
            "no_code": len(rs) - len(coded),
            "clean_raw_pct": pct(sum(r["clean_raw"] for r in coded), len(coded)),
            "clean_fmt_pct": pct(sum(r["clean_fmt"] for r in coded), len(coded)),
            "errors_per_answer_fmt": round(sum(len(r["codes_fmt"]) for r in coded) / len(coded), 2) if coded else 0.0,
            "codes_raw": dict(sorted(raw_c.items())),
            "codes_fmt": dict(sorted(fmt_c.items())),
            "failing_fmt": [{"name": r["name"], "code": r["first_code"], "msg": r["first_msg"]}
                            for r in coded if not r["clean_fmt"]],
        })
    return out


def render(report):
    lines = []
    for g in report:
        lines.append(f"== {g['file']} / {g['condition']}")
        lines.append(f"answers {g['answers']}  no-code {g['no_code']}  clean raw {g['clean_raw_pct']}%  "
                     f"clean after fmt {g['clean_fmt_pct']}%  errors/answer after fmt {g['errors_per_answer_fmt']}")
        lines.append("code     raw  fmt")
        for c in sorted(set(g["codes_raw"]) | set(g["codes_fmt"])):
            lines.append(f"{c:<8} {g['codes_raw'].get(c, 0):>3}  {g['codes_fmt'].get(c, 0):>3}")
        for f in g["failing_fmt"]:
            lines.append(f"fail {f['name']} {f['code']} {f['msg']}")
        lines.append("")
    return "\n".join(lines)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("files", nargs="*")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--no-default", action="store_true", help="only the given files")
    ap.add_argument("--out", help="write the report here instead of stdout")
    a = ap.parse_args(argv)
    binary = aleph_bin()
    if not os.path.exists(binary):
        sys.exit(f"aleph binary not found: {binary} (set ALEPH)")
    report = build_report(([] if a.no_default else default_inputs()) + a.files, binary)
    text = json.dumps(report, sort_keys=True, indent=1) if a.json else render(report)
    if a.out:
        with open(a.out, "w", encoding="utf-8") as f:
            f.write(text + "\n")
    else:
        print(text)


if __name__ == "__main__":
    main()
