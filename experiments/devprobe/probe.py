"""Dev probe: build prompts, check answers (compile + hidden examples)."""
import collections, json, os, re, subprocess, sys, tempfile
ALEPH = os.environ.get("ALEPH", os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../target/debug/aleph"))
HERE = os.path.dirname(os.path.abspath(__file__))
TMP = tempfile.mkdtemp(prefix="devprobe-")

TASKS = os.path.join(HERE, "tasks.txt")


def tasks():
    out = []
    for line in open(TASKS):
        if line.startswith("#") or not line.strip():
            continue
        f = line.rstrip("\n").split("|")
        name, stmt, header, vis, hid = f[0], f[1], "|".join(f[2:-2]), f[-2], f[-1]
        out.append(dict(name=name, stmt=stmt, header=header.replace("\\n", "\n"),
                        vis=vis.split(";"), hid=hid.split(";")))
    return out

def prompt():
    parts = []
    for t in tasks():
        ex = "\n".join(" ex " + e for e in t["vis"])
        parts.append(f"### {t['name']}\n{t['stmt']}\n```\n{t['header']}\n{ex}\n```")
    return "\n\n".join(parts)

def blocks(text):
    """name -> code, from '### name' headings followed by a fenced block."""
    res = {}
    for m in re.finditer(r"###\s*(\w+)\s*\n+```(?:aleph)?\n(.*?)```", text, re.S):
        res[m.group(1)] = m.group(2)
    return res

def check(code, path):
    open(path, "w").write(code)
    p = subprocess.run([ALEPH, "check", path], capture_output=True, text=True, timeout=60)
    diags = [json.loads(l) for l in p.stdout.splitlines() if l.strip().startswith("{")]
    return [d for d in diags if d.get("severity") != "warning"]

def inject(code, t):
    lines = code.split("\n")
    head = t["header"].split("\n")[-1].split(" ->")[0]
    for i, l in enumerate(lines):
        if l.startswith(head + " ") or l.startswith(head + "->") or l == head:
            return "\n".join(lines[:i+1] + [" ex " + e for e in t["hid"]] + lines[i+1:])
    return None

def main(answer_file):
    ans = blocks(open(answer_file).read())
    codes = collections.Counter(); clean = passed = 0; errs = 0
    for t in tasks():
        code = ans.get(t["name"])
        if code is None:
            print(f"{t['name']:10} NO ANSWER"); continue
        d = check(code, os.path.join(TMP, t["name"] + ".aph"))
        errs += len(d)
        for x in d: codes[x["code"]] += 1
        if d:
            print(f"{t['name']:10} FAIL " + "; ".join(f"{x['code']} l{x.get('line')} {x['msg'][:60]} exp={x.get('expected','')[:40]} found={x.get('found','')[:40]}" for x in d[:3]))
            continue
        clean += 1
        inj = inject(code, t)
        h = check(inj, os.path.join(TMP, t["name"] + "_h.aph")) if inj else [{"code": "NOHEADER"}]
        if h:
            print(f"{t['name']:10} clean, hidden FAIL " + "; ".join(f"{x['code']} {x.get('expected','')} {x.get('found','')}" for x in h[:2]))
        else:
            passed += 1
            print(f"{t['name']:10} ok")
    n = len(tasks())
    print(f"clean {clean}/{n}  hidden-pass {passed}/{n}  errors {errs} ({errs/n:.2f}/problem)  {dict(codes)}")

if __name__ == "__main__":
    args = sys.argv[1:]
    if len(args) == 3 and args[0] == "--tasks":
        TASKS = os.path.abspath(args[1])
        args = args[2:]
    if len(args) != 1:
        sys.exit("usage: probe.py [--tasks FILE] prompt | ANSWERS.md")
    if args[0] == "prompt":
        print(prompt())
    else:
        main(args[0])
