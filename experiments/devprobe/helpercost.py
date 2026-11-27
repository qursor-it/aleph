"""Share of answer characters spent on helper `ex` lines and helper signature types.

usage: python3 helpercost.py ANSWERS..
A helper is a fn called from another fn of the same answer. Signature types are
estimated with a regex (rough, upper bound).
"""
import re,sys
BLOCK=r"###\s*(\w+)\s*\n+```(?:aleph)?\n(.*?)```"
tot=hex_=hty=allex=0; nh=0
for p in sys.argv[1:]:
    for m in re.finditer(BLOCK,open(p).read(),re.S):
        code=m.group(2); fns=[]; cur=None
        for l in code.splitlines():
            if l.startswith("fn "): cur=[l]; fns.append(cur)
            elif cur is not None and l.strip(): cur.append(l)
        tot+=len("\n".join(l for l in code.splitlines() if l.strip()))
        names=[f[0].split()[1] for f in fns]
        for f,n in zip(fns,names):
            exs=sum(len(l)+1 for l in f[1:] if l.lstrip().startswith("ex "))
            allex+=exs
            others=" ".join(" ".join(g[1:]) for g,k in zip(fns,names) if k!=n)
            if n!="main" and re.search(r"(?<![\w.])%s(?![\w])"%re.escape(n),others):
                nh+=1; hex_+=exs
                sig=f[0]; head=sig.split(" -> ")[0]
                hty+=sum(len(t)+1 for t in re.findall(r":(\S+(?: (?:i64|f64|str|bool|unit|list|opt|res|dict|json|\w+))*)",head))  # rough
                if " -> " in sig: hty+=len(sig.split(" -> ",1)[1])+4
print(f"chars {tot}; all ex {allex} ({allex/tot:.1%}); helpers {nh}: ex {hex_} ({hex_/tot:.1%}), sig types ~{hty} ({hty/tot:.1%})")
