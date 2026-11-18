"""What-if count: the HumanEval corpus rewritten with candidate features.

B: builtins abs min max starts uniq map2, and @ on builtins.
C: B plus inline lambdas `lam x.. BODY` that may read the enclosing params
   and lets; arity from the expected function type.
Not compiled (the features do not exist yet): only counted. Same normalisation
as count.py (no doc, no ex)."""
import json, os, statistics, sys
import tiktoken
sys.path.insert(0, os.path.dirname(__file__))
from solutions import S

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
enc = tiktoken.get_encoding("o200k_base").encode

B = dict(S)
B[15] = ' join map @show range 0 + n 1 " "\n'
B[16] = ' len uniq split lower string ""\n'
B[29] = S[29].replace(' let starts == join slice split h "" 0 count p "" p\n', "").replace("if starts push", "if starts h p push")
B[34] = " uniq l\n"
B[35] = " fold @max else at l 0 0 l\n"
B[97] = " * % abs a 10 % abs b 10\n"
B[126] = " and == show lst show sort lst <= fold @max 0 vals fold @bump dict i64 i64 lst 2\n" + S[126].split("fn bump", 1)[1].join(["fn bump", ""])
B[152] = """ map2 @absdiff game guess
fn absdiff a:i64 b:i64 -> i64
 abs - a b
"""
B[163] = """ filter @even range max 2 min a b + min 8 max a b 1
fn even x:i64 -> bool
 == % x 2 0
"""

C = dict(B)
C.update({
    5: " slice fold lam acc x push push acc x delimeter empty i64 numbers 0 - * 2 len numbers 1\n",
    7: " filter lam s contains s substring strings\n",
    9: " map lam i fold @max else at numbers 0 0 slice numbers 0 + i 1 range 0 len numbers\n",
    14: ' map lam n join slice split string "" 0 n "" range 1 + count string 1\n',
    18: ' let cs split string ""\n len filter lam i == join slice cs i + i count substring "" substring range 0 + - len cs count substring 1\n',
    24: " fold @max 1 filter lam i == % n i 0 range 1 n\n",
    26: " filter lam x == len filter lam y == x y numbers 1 numbers\n",
    29: " filter lam s starts s prefix strings\n",
    30: " filter lam x > x 0 l\n",
    31: " and >= n 2 == 0 len filter lam k == % n k 0 range 2 n\n",
    36: ' sum map lam i len filter lam c == c "7" split show i "" filter lam i or == % i 11 0 == % i 13 0 range 0 n\n',
    42: " map lam x + x 1 l\n",
    43: " > len filter lam i > len filter lam j == 0 + else at l i 0 else at l j 0 range + i 1 len l 0 range 0 len l 0\n",
    52: " == 0 len filter lam x >= x t l\n",
    58: " uniq filter lam x > len filter lam y == x y l2 0 l1\n",
    62: " map lam i * i else at xs i 0 range 1 len xs\n",
    64: ' + len filter lam c contains "aeiouAEIOU" c split s "" if == lower else at split s "" - count s 1 "" "y" 1 0\n',
    73: " len filter lam i != else at arr i 0 else at arr - - len arr 1 i 0 range 0 / len arr 2\n",
    85: " sum filter lam x == % x 2 0 map lam i else at lst i 0 filter lam i == % i 2 1 range 0 len lst\n",
    96: " filter lam m == 0 len filter lam k == % m k 0 range 2 m range 2 n\n",
    100: " map lam i + n * 2 i range 0 n\n",
    106: " map lam i if == % i 2 0 fold lam a b * a b 1 range 1 + i 1 sum range 1 + i 1 range 1 + n 1\n",
    121: " sum filter lam x != % x 2 0 map lam i else at lst i 0 filter lam i == % i 2 0 range 0 len lst\n",
    122: " sum filter lam x <= count show x 2 slice arr 0 k\n",
    126: " and == show lst show sort lst <= fold @max 0 map lam x len filter lam y == x y lst lst 2\n",
    131: ' let ds filter lam d == % d 2 1 map lam c else int c 0 split show n ""\n if == len ds 0 0 fold lam a b * a b 1 ds\n',
    139: " fold lam a b * a b 1 map lam k fold lam a b * a b 1 range 1 + k 1 range 1 + n 1\n",
    142: """ sum map lam i term else at lst i 0 i range 0 len lst
fn term x:i64 i:i64 -> i64
 if == % i 3 0 * x x if == % i 4 0 * * x x x x
""",
    150: " if and > n 1 == 0 len filter lam k == % n k 0 range 2 n x y\n",
    152: " map2 lam a b abs - a b game guess\n",
    163: " filter lam x == % x 2 0 range max 2 min a b + min 8 max a b 1\n",
})

prompts = {json.loads(l)["original"].split("/")[1]: json.loads(l)["prompt"] for l in open(f"{REPO}/experiments/benchmarks/humaneval/humaneval-aleph.jsonl")}
py = {p["id"]: p["o200k"]["python"] for p in json.load(open(os.path.join(os.path.dirname(__file__), "tokens.json")))["pairs"]}


def logic(src):
    return "\n".join(l for l in src.splitlines() if not l.startswith((" doc ", " ex "))) + "\n"


out = {}
for name, var in (("A today", S), ("B builtins", B), ("C builtins+lambdas", C)):
    r = [len(enc(logic(prompts[str(k)] + var[k]))) / py[f"HumanEval/{k}"] for k in S]
    helpers = statistics.mean(logic(prompts[str(k)] + var[k]).count("\nfn ") for k in S)
    out[name] = {"median": round(statistics.median(r), 3), "pooled": round(sum(len(enc(logic(prompts[str(k)] + var[k]))) for k in S) / sum(py[f"HumanEval/{k}"] for k in S), 3), "smaller": sum(x < 1 for x in r), "helpers": round(helpers, 2)}
    print(f"{name:20} median {out[name]['median']:.2f} pooled {out[name]['pooled']:.2f} shorter {out[name]['smaller']}/61 helpers {out[name]['helpers']}")
json.dump(out, open(os.path.join(os.path.dirname(__file__), "whatif.json"), "w"), indent=1)
