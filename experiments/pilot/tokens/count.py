"""Pilot token count: the same verified programs in Aleph and Python."""
import ast, gzip, json, os, statistics, subprocess, sys, tempfile, random
import tiktoken
from tokenizers import Tokenizer
sys.path.insert(0, os.path.dirname(__file__))
from solutions import S

VOCAB = os.environ.get("VOCAB", ".")
ALEPH = os.environ.get("ALEPH", "aleph")
REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
TOK = {
    "o200k": tiktoken.get_encoding("o200k_base").encode,
    "cl100k": tiktoken.get_encoding("cl100k_base").encode,
    "deepseek-v3": (lambda t, k=Tokenizer.from_file(os.path.join(VOCAB, "DeepSeek-V3.json")): k.encode(t, add_special_tokens=False).ids),
    "qwen2.5-coder": (lambda t, k=Tokenizer.from_file(os.path.join(VOCAB, "Qwen2.5-Coder-7B-Instruct.json")): k.encode(t, add_special_tokens=False).ids),
}


def py_norm(src):
    tree = ast.parse(src)
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Module)):
            b = node.body
            if b and isinstance(b[0], ast.Expr) and isinstance(b[0].value, ast.Constant) and isinstance(b[0].value.value, str):
                node.body = b[1:] or [ast.Pass()]
    return ast.unparse(tree) + "\n"


def aleph_fmt(src):
    with tempfile.NamedTemporaryFile("w", suffix=".aph", delete=False) as f:
        f.write(src)
    out = subprocess.run([ALEPH, "fmt", f.name], capture_output=True, text=True)
    os.unlink(f.name)
    assert out.returncode == 0, out.stdout + out.stderr
    return out.stdout


def aleph_variants(src):
    lines = [l for l in src.splitlines() if not l.startswith(" doc ")]
    logic = [l for l in lines if not l.startswith(" ex ")]
    return "\n".join(logic) + "\n", "\n".join(lines) + "\n"


pairs = []
he = {json.loads(l)["task_id"].split("/")[1]: json.loads(l) for l in gzip.open(os.path.join(VOCAB, "HumanEval.jsonl.gz"), "rt")}
al = {json.loads(l)["original"].split("/")[1]: json.loads(l) for l in open(f"{REPO}/experiments/benchmarks/humaneval/humaneval-aleph.jsonl")}
for k, body in S.items():
    k = str(k)
    py = py_norm(he[k]["prompt"] + he[k]["canonical_solution"])
    a_logic, a_ex = aleph_variants(aleph_fmt(al[k]["prompt"] + body))
    pairs.append({"id": f"HumanEval/{k}", "set": "humaneval", "python": py, "aleph": a_logic, "aleph_ex": a_ex})
for name in sorted(os.listdir(f"{REPO}/bench/ref/py")):
    n = name[:-3]
    p = f"{REPO}/tests/aph/bench_{n}.aph"
    if not os.path.exists(p):
        p = f"{REPO}/bench/heavy/{n}.aph"
    if not os.path.exists(p):
        continue
    py = py_norm(open(f"{REPO}/bench/ref/py/{name}").read())
    a_logic, a_ex = aleph_variants(open(p).read())
    pairs.append({"id": f"bench/{n}", "set": "bench", "python": py, "aleph": a_logic, "aleph_ex": a_ex})

for p in pairs:
    for t, enc in TOK.items():
        p[t] = {"python": len(enc(p["python"])), "aleph": len(enc(p["aleph"])), "aleph_ex": len(enc(p["aleph_ex"]))}


def boot_ci(xs, n=2000):
    rnd = random.Random(0)
    meds = sorted(statistics.median(rnd.choices(xs, k=len(xs))) for _ in range(n))
    return meds[int(0.025 * n)], meds[int(0.975 * n)]


summary = {}
for subset in ("humaneval", "bench", "all"):
    ps = [p for p in pairs if subset == "all" or p["set"] == subset]
    summary[subset] = {"programs": len(ps)}
    for t in TOK:
        for v in ("aleph", "aleph_ex"):
            r = [p[t][v] / p[t]["python"] for p in ps]
            tot = sum(p[t][v] for p in ps) / sum(p[t]["python"] for p in ps)
            lo, hi = boot_ci(r)
            summary[subset][f"{t}/{v}"] = {"median": round(statistics.median(r), 3), "ci95": [round(lo, 3), round(hi, 3)], "pooled": round(tot, 3), "smaller": sum(x < 1 for x in r)}
json.dump({"pairs": pairs, "summary": summary}, open(os.path.join(os.path.dirname(__file__), "tokens.json"), "w"), indent=1)
for s, d in summary.items():
    print(s, d["programs"])
    for k, v in d.items():
        if k != "programs":
            print(f"  {k:24} median {v['median']:.2f} ci {v['ci95']} pooled {v['pooled']:.2f} smaller {v['smaller']}")
llms = open(f"{REPO}/llms.txt").read()
print("llms.txt", {t: len(e(llms)) for t, e in TOK.items()})
