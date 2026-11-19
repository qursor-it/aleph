"""T-050: run the what-if variants of whatif.py against the hidden HumanEval tests.

whatif.py counted variants B (ADR-0036 builtins) and C (B + lambdas, ADR-0037)
without compiling them. Now both features exist, so each variant program is
formatted and tested like a model completion (evaluate.py). Standard library
only: the dicts are read from whatif.py's source without running its counting.

    ALEPH=target/release/aleph python3 experiments/pilot/tokens/verify_whatif.py
"""
import ast, json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../../.."))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(REPO, "experiments/benchmarks/humaneval"))
from solutions import S  # noqa: E402
import evaluate  # noqa: E402


def variants():
    """Execute only whatif.py's statements that build B and C."""
    tree = ast.parse(open(os.path.join(HERE, "whatif.py"), encoding="utf-8").read())
    keep = []
    for node in tree.body:
        targets = getattr(node, "targets", [])
        names = {t.id if isinstance(t, ast.Name) else getattr(t.value, "id", None) for t in targets}
        if names & {"B", "C"}:
            keep.append(node)
    scope = {"S": S, "dict": dict}
    exec(compile(ast.Module(body=keep, type_ignores=[]), "whatif.py", "exec"), scope)
    return {"A today": S, "B builtins": scope["B"], "C builtins+lambdas": scope["C"]}


def main():
    records = {}
    with open(os.path.join(REPO, "experiments/benchmarks/humaneval/humaneval-aleph.jsonl"), encoding="utf-8") as f:
        for line in f:
            record = json.loads(line)
            records[record["original"].split("/")[1]] = record
    report = {}
    for name, var in variants().items():
        results = {}
        for k in S:
            status, detail = evaluate.run(evaluate.program(records[str(k)], var[k]))
            results[str(k)] = status if status == "pass" else {"status": status, "detail": detail[:400]}
        passed = sum(r == "pass" for r in results.values())
        print(f"{name:20} {passed}/{len(S)} pass")
        for k, r in results.items():
            if r != "pass":
                print(f"  {k}: {r['status']}: {r['detail'][:200]}")
        report[name] = {"passed": passed, "total": len(S), "results": results}
    json.dump(report, open(os.path.join(HERE, "verify_whatif.json"), "w"), indent=1, sort_keys=True)


if __name__ == "__main__":
    main()
