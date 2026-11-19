#!/usr/bin/env python3
"""Translate HumanEval into Aleph, in the style of MultiPL-E.

Usage:
  python3 humaneval_to_aleph.py SOURCE OUT.jsonl

SOURCE is either MultiPL-E's folder of typed HumanEval problems
(datasets/originals-with-cleaned-doctests, one HumanEval_N_name.py per
problem, with type annotations added by the MultiPL-E authors) or the original
HumanEval.jsonl[.gz] (mostly untyped, so most problems are skipped).

Each problem becomes a prompt (record types for tuples, the function header,
a `doc` line with the description, and the docstring examples as `ex` lines)
and a set of hidden tests (`ex` lines translated from `assert candidate(...)
== value`). Records follow MultiPL-E's fields (name, language, prompt,
doctests, original, prompt_terminology, tests, stop_tokens) plus `entry_point`
and `stub`, a completion of the right type used to validate the translation.

Translation is type directed, from the Python annotations. Problems whose
types or tests cannot be expressed are skipped and listed in skipped.json with
the reason, as MultiPL-E does for its languages. Python 3.10+, standard
library only.
"""
import ast
import decimal
import gzip
import json
import pathlib
import re
import sys

KEYWORDS = set("fn type use let do ex doc if try some none ok err else has isok empty nil true false and or not neg match lam".split())
BUILTINS = set("abs args at cat chr contains count del eprint filter float fold get int jbool jdict jint jlist jnum join jparse json jstr keys len lower map map2 max min ord print push put range read rev show size slice sort split starts ends sum trim uniq upper vals write".split())
TYPE_NAMES = {"i64", "f64", "bool", "str", "unit", "list", "opt", "res", "dict", "main"}
RESERVED = KEYWORDS | BUILTINS | TYPE_NAMES
# Unlike MultiPL-E, a new top-level `fn` does not stop the completion: Aleph has
# no lambdas, so helper functions are part of a solution.
STOP_TOKENS = ["\n```", "\nfn main"]


class Skip(Exception):
    """The problem cannot be expressed in Aleph v0.2."""


def ident(name: str) -> str:
    out = re.sub(r"[^a-z0-9]", "", name.lower())
    if not out or not out[0].isalpha():
        out = "x" + out
    while out in RESERVED:
        out += "x"
    return out


class Types:
    """Python annotations to Aleph types; tuples become record types."""

    def __init__(self):
        self.records = {}  # tuple of aleph field types -> record name

    def of(self, node) -> tuple:
        match node:
            case None:
                raise Skip("untyped parameter or result")
            case ast.Name("int"):
                return ("i64",)
            case ast.Name("float"):
                return ("f64",)
            case ast.Name("str"):
                return ("str",)
            case ast.Name("bool"):
                return ("bool",)
            case ast.Subscript(ast.Name("List" | "list"), elem):
                return ("list", self.of(elem))
            case ast.Subscript(ast.Name("Optional"), elem):
                return ("opt", self.of(elem))
            case ast.Subscript(ast.Name("Dict" | "dict"), ast.Tuple([key, value])):
                key_type = self.of(key)
                if key_type not in (("i64",), ("bool",), ("str",)):
                    raise Skip("dict key type not i64, bool or str")
                return ("dict", key_type, self.of(value))
            case ast.Subscript(ast.Name("Tuple" | "tuple"), ast.Tuple(elts)):
                if any(isinstance(e, ast.Constant) and e.value is Ellipsis for e in elts):
                    raise Skip("variable-length tuple")
                fields = tuple(self.of(e) for e in elts)
                if fields not in self.records:
                    self.records[fields] = f"tup{len(self.records)}"
                return ("rec", self.records[fields], fields)
            case ast.Name("Any") | ast.Subscript(ast.Name("Union"), _):
                raise Skip("Any or Union type")
            case _:
                raise Skip(f"unsupported annotation {ast.unparse(node)}")

    def render(self, ty: tuple) -> str:
        match ty:
            case ("list", elem):
                return f"list {self.render(elem)}"
            case ("opt", elem):
                return f"opt {self.render(elem)}"
            case ("dict", key, value):
                return f"dict {self.render(key)} {self.render(value)}"
            case ("rec", name, _):
                return name
            case (simple,):
                return simple
        raise AssertionError(ty)

    def declarations(self) -> list:
        lines = []
        for fields, name in sorted(self.records.items(), key=lambda item: item[1]):
            params = " ".join(f"{chr(ord('a') + i)}:{self.render(f)}" for i, f in enumerate(fields))
            lines.append(f"type {name} {params}")
        return lines


def string_literal(value: str) -> str:
    out = []
    for ch in value:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 32 or ord(ch) == 127:
            raise Skip("control character in a string")
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def float_literal(value: float) -> str:
    if value != value or value in (float("inf"), float("-inf")):
        raise Skip("NaN or infinity")
    text = format(decimal.Decimal(repr(float(value))), "f")
    if "." not in text:
        text += ".0"
    if text.startswith("-0") and float(text) == 0:
        text = text[1:]
    return text


class Values:
    """Python literal values to Aleph expressions, directed by the type."""

    def __init__(self, types: Types):
        self.types = types

    def expr(self, value, ty: tuple) -> str:
        match ty:
            case ("i64",):
                if isinstance(value, bool) or not isinstance(value, int):
                    raise Skip("non-integer value for int")
                if not -(2**63) <= value < 2**63:
                    raise Skip("integer outside i64")
                return str(value)
            case ("f64",):
                if isinstance(value, bool) or not isinstance(value, (int, float)):
                    raise Skip("non-numeric value for float")
                return float_literal(value)
            case ("str",):
                if not isinstance(value, str):
                    raise Skip("non-string value for str")
                return string_literal(value)
            case ("bool",):
                if not isinstance(value, bool):
                    raise Skip("non-bool value for bool")
                return "true" if value else "false"
            case ("list", elem):
                if not isinstance(value, (list, tuple)):
                    raise Skip("non-list value for list")
                if not value:
                    return f"empty {self.types.render(elem)}"
                return "[" + " ".join(self.expr(v, elem) for v in value) + "]"
            case ("opt", elem):
                if value is None:
                    return f"none {self.types.render(elem)}"
                return f"some {self.expr(value, elem)}"
            case ("dict", key, val):
                if not isinstance(value, dict):
                    raise Skip("non-dict value for dict")
                out = f"dict {self.types.render(key)} {self.types.render(val)}"
                for k, v in value.items():
                    out = f"put {out} {self.expr(k, key)} {self.expr(v, val)}"
                return out
            case ("rec", name, fields):
                if not isinstance(value, (tuple, list)) or len(value) != len(fields):
                    raise Skip("tuple value of the wrong length")
                return name + " " + " ".join(self.expr(v, f) for v, f in zip(value, fields))
        raise AssertionError(ty)


def find_function(prompt: str, entry_point: str) -> ast.FunctionDef:
    module = ast.parse(prompt)
    for node in module.body:
        if isinstance(node, ast.FunctionDef) and node.name == entry_point:
            return node
    raise Skip("entry point not found in prompt")


def literal(node):
    try:
        return ast.literal_eval(node)
    except (ValueError, SyntaxError, TypeError) as error:
        raise Skip(f"non-literal value {ast.unparse(node)[:40]}") from error


def example_lines(calls, fn_name, arg_types, ret_type, values, types):
    """(args nodes, expected node) pairs to `ex` lines; a pair that cannot be
    expressed is dropped, never approximated."""
    lines, dropped = [], 0
    for args, expected in calls:
        try:
            if len(args) != len(arg_types):
                raise Skip("wrong number of arguments")
            rendered = [values.expr(literal(a), t) for a, t in zip(args, arg_types)]
            want = literal(expected)
            if ret_type[0] == "dict":
                if not isinstance(want, dict):
                    raise Skip("non-dict expected value")
                key, val = ret_type[1], ret_type[2]
                pairs = sorted(want.items())
                call = " ".join([fn_name] + rendered)
                keys = values.expr([k for k, _ in pairs], ("list", key))
                vals = values.expr([v for _, v in pairs], ("list", val))
                lines.append(f" ex keys {call} = {keys}")
                lines.append(f" ex vals {call} = {vals}")
                continue
            lines.append(" ex " + " ".join([fn_name] + rendered) + " = " + values.expr(want, ret_type))
        except Skip:
            dropped += 1
    return lines, dropped


def test_calls(test_source: str):
    """`assert candidate(ARGS) == EXPECTED` pairs from HumanEval's check()."""
    calls, other = [], 0
    for node in ast.walk(ast.parse(test_source)):
        if not isinstance(node, ast.Assert):
            continue
        test = node.test
        if (isinstance(test, ast.Compare) and len(test.ops) == 1 and isinstance(test.ops[0], ast.Eq)
                and isinstance(test.left, ast.Call) and isinstance(test.left.func, ast.Name)
                and test.left.func.id == "candidate" and not test.left.keywords):
            calls.append((test.left.args, test.comparators[0]))
        elif isinstance(test, ast.Constant) and test.value is True:
            continue
        else:
            other += 1
    return calls, other


DOCTEST_CALL = re.compile(r"^\s*>>>\s*(.+)$")


def split_docstring(doc: str, entry_point: str):
    """Description text, and (call, expected) doctest pairs on the entry point."""
    text_lines, pairs, lines, i = [], [], doc.splitlines(), 0
    while i < len(lines):
        match = DOCTEST_CALL.match(lines[i])
        if match:
            call = match.group(1).strip()
            expected = lines[i + 1].strip() if i + 1 < len(lines) else ""
            i += 2
            try:
                call_node = ast.parse(call, mode="eval").body
                want_node = ast.parse(expected, mode="eval").body
            except SyntaxError:
                continue
            if isinstance(call_node, ast.Call) and isinstance(call_node.func, ast.Name) and call_node.func.id == entry_point:
                pairs.append((call_node.args, want_node))
            continue
        text_lines.append(lines[i])
        i += 1
    text = re.sub(r"\n{3,}", "\n\n", "\n".join(l.rstrip() for l in text_lines)).strip()
    # Examples written as "f(x) == y" or "f(x) ➞ y" inside prose stay in the text.
    return text, pairs


def default_value(ty: tuple, types: Types) -> str:
    match ty:
        case ("i64",):
            return "0"
        case ("f64",):
            return "0.0"
        case ("str",):
            return '""'
        case ("bool",):
            return "false"
        case ("list", elem):
            return f"empty {types.render(elem)}"
        case ("opt", elem):
            return f"none {types.render(elem)}"
        case ("dict", key, val):
            return f"dict {types.render(key)} {types.render(val)}"
        case ("rec", name, fields):
            return name + " " + " ".join(default_value(f, types) for f in fields)
    raise AssertionError(ty)


def translate(problem: dict) -> dict:
    entry = problem["entry_point"]
    fn = find_function(problem["prompt"], entry)
    if fn.args.vararg or fn.args.kwarg or fn.args.kwonlyargs or fn.args.defaults:
        raise Skip("varargs, keyword-only or default arguments")
    types, = (Types(),)
    values = Values(types)
    arg_types = [types.of(a.annotation) for a in fn.args.args]
    ret_type = types.of(fn.returns)
    name = ident(entry)
    params, used = [], {name}
    for arg, ty in zip(fn.args.args, arg_types):
        param = ident(arg.arg)
        while param in used:
            param += "x"
        used.add(param)
        params.append(f"{param}:{types.render(ty)}")

    doc = ast.get_docstring(fn, clean=True) or ""
    description, doc_pairs = split_docstring(doc, entry)
    description = re.sub(r"\b" + re.escape(entry) + r"\b", name, description)
    if not description:
        raise Skip("empty description")
    doc_examples, _ = example_lines(doc_pairs, name, arg_types, ret_type, values, types)

    calls, other = test_calls(problem["test"])
    tests, dropped = example_lines(calls, name, arg_types, ret_type, values, types)
    if not tests:
        raise Skip("no test could be translated")

    header = f"fn {name} {' '.join(params)} -> {types.render(ret_type)}".replace("  ", " ")
    prompt_lines = types.declarations() + [header, " doc " + string_literal(description)] + doc_examples
    number = problem["task_id"].split("/")[1]
    return {
        "name": f"HumanEval_{number}_{entry}",
        "language": "aleph",
        "prompt": "\n".join(prompt_lines) + "\n",
        "doctests": "transform",
        "original": f"HumanEval/{number}",
        "prompt_terminology": "verbatim",
        "tests": "\n".join(tests) + "\n",
        "stop_tokens": STOP_TOKENS,
        "entry_point": name,
        "stub": " " + default_value(ret_type, types) + "\n",
        "tests_total": len(calls) + other,
        "tests_translated": len(tests) if ret_type[0] != "dict" else len(tests) // 2,
    }


def load(path: str):
    source = pathlib.Path(path)
    if source.is_dir():
        problems = []
        for file in sorted(source.glob("HumanEval_*.py"), key=lambda f: int(f.name.split("_")[1])):
            text = file.read_text(encoding="utf-8")
            prompt = text.split("### Canonical solution below ###")[0]
            test = text.split("### Unit tests below ###")[1] if "### Unit tests below ###" in text else ""
            number, entry = file.stem.split("_", 2)[1:]
            problems.append({"task_id": f"HumanEval/{number}", "prompt": prompt, "entry_point": entry, "test": test})
        return problems
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]


def main(argv):
    if len(argv) != 2:
        raise SystemExit(__doc__)
    problems = load(argv[0])
    out, skipped = [], []
    for problem in problems:
        try:
            out.append(translate(problem))
        except Skip as reason:
            skipped.append({"task_id": problem["task_id"], "reason": str(reason)})
    target = pathlib.Path(argv[1])
    target.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in out), encoding="utf-8")
    (target.parent / "skipped.json").write_text(json.dumps(skipped, indent=2) + "\n", encoding="utf-8")
    print(f"{len(out)} translated, {len(skipped)} skipped of {len(problems)}")


if __name__ == "__main__":
    main(sys.argv[1:])
