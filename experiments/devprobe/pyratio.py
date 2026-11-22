"""Same model, same tasks: size of its Aleph answers vs its Python answers.

usage: python3 pyratio.py PY_ANSWERS ALEPH_ANSWERS..
Python blocks run with their own asserts (python3 -I, 10 s); Aleph answers
are the recorded runs (score them with probe.py first). Sizes are characters
of logic only: Aleph without `ex` lines, Python without `assert` lines.
Tokens are estimated as characters / 4 (no real tokenizer offline).
"""
import os, re, statistics, subprocess, sys, tempfile

BLOCK = r"###\s*(\w+)\s*\n+```(?:{lang})?\n(.*?)```"


def blocks(path, lang):
    return {m.group(1): m.group(2) for m in re.finditer(BLOCK.format(lang=lang), open(path).read(), re.S)}


def py_passes(code):
    with tempfile.TemporaryDirectory() as d:
        f = os.path.join(d, "t.py")
        open(f, "w").write(code)
        try:
            return subprocess.run([sys.executable, "-I", f], cwd=d, capture_output=True, timeout=10).returncode == 0
        except subprocess.TimeoutExpired:
            return False


def logic(code, drop):
    return "\n".join(l for l in code.splitlines() if l.strip() and not l.lstrip().startswith(drop))


def main(py_path, aleph_paths):
    py = blocks(py_path, "python")
    al = {}
    for p in aleph_paths:
        al.update(blocks(p, "aleph"))
    rows = []
    for name, code in py.items():
        if name not in al:
            continue
        ok = py_passes(code)
        a, b = len(logic(al[name], "ex ")), len(logic(code, "assert "))
        rows.append((name, ok, a, b))
        print(f"{name:10} py {'ok  ' if ok else 'FAIL'} aleph {a:4} python {b:4} ratio {a / b:.2f}")
    good = [r for r in rows if r[1]]
    ratios = [r[2] / r[3] for r in good]
    a, b = sum(r[2] for r in good), sum(r[3] for r in good)
    print(f"python passes {len(good)}/{len(rows)}; on those: aleph {a} chars (~{a // 4} tokens), "
          f"python {b} (~{b // 4}); total ratio {a / b:.3f}, median per task {statistics.median(ratios):.3f}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
