#!/usr/bin/env python3
"""EXP-003 token corpus: verify every language version and report token counts.

Usage:
  corpus.py verify CORPUS_DIR
  corpus.py report CORPUS_DIR [--vocab PATH]... [--json] [--seed 0]

Python 3 standard library only, offline.  See README.md for the layout.

The Aleph binary comes from $ALEPH, else ``target/debug/aleph`` in the repo.
"""
import argparse
import ast
import contextlib
import json
import math
import os
import pathlib
import random
import shutil
import statistics
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[2]
DEFAULT_ALEPH = str(REPO / "target" / "debug" / "aleph")
SUITE_DIR = HERE.parent / "suite"
if str(SUITE_DIR) not in sys.path:
    sys.path.insert(0, str(SUITE_DIR))
import suite  # noqa: E402  (path set above)

LANGS = ("aleph", "python", "bash", "c")
LANGUAGE_FILES = {
    "aleph": "aleph.aph",
    "python": "python.py",
    "bash": "bash.sh",
    "c": "c.c",
}
# The debug interpreter needs well over the suite default of 10 s for
# `bench_fib.aph` (fib 30 by naive recursion), so the corpus is generous.
DEFAULT_TIME_LIMIT = 120
DEFAULT_BOOTSTRAP = 2000
EXACT_WILCOXON_LIMIT = 20


class TokenCountError(Exception):
    """`aleph tokens` failed."""


# ---------------------------------------------------------------------------
# Corpus discovery and verification


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def discover_programs(corpus_dir):
    """Program folders, sorted by name."""
    corpus_dir = pathlib.Path(corpus_dir).resolve()
    return sorted(
        path
        for path in corpus_dir.iterdir()
        if path.is_dir()
        and (path / "tests.json").is_file()
        and (path / "meta.json").is_file()
        and not path.name.startswith(".")
    )


def _safe_rel(path):
    if not isinstance(path, str) or not path:
        return False
    candidate = pathlib.PurePosixPath(path)
    return not candidate.is_absolute() and ".." not in candidate.parts


def _write_file(base, rel, content):
    if not _safe_rel(rel):
        raise ValueError(f"unsafe relative path: {rel!r}")
    path = pathlib.Path(base) / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


@contextlib.contextmanager
def _suite_task_dir(program_dir):
    """A temporary EXP-002 task dir with the program's tests and inputs.

    ``suite.run_tests`` insists on a ``task.json``; the corpus stores the task
    metadata in ``meta.json`` instead, so a minimal task is materialised here.
    """
    with tempfile.TemporaryDirectory() as directory:
        task = pathlib.Path(directory)
        (task / "task.json").write_text(
            json.dumps({"time_limit_s": DEFAULT_TIME_LIMIT}), encoding="utf-8"
        )
        shutil.copy2(program_dir / "tests.json", task / "tests.json")
        inputs = program_dir / "inputs"
        if inputs.is_dir():
            shutil.copytree(inputs, task / "inputs", dirs_exist_ok=True)
        yield task


def run_c_tests(program_dir, source, tests, limit=DEFAULT_TIME_LIMIT):
    """Compile one C program and run it against the shared tests."""
    program_dir = pathlib.Path(program_dir)
    source = pathlib.Path(source)
    with tempfile.TemporaryDirectory() as directory:
        binary = pathlib.Path(directory) / "program"
        compile_command = [
            "cc",
            "-std=c99",
            "-O2",
            "-o",
            str(binary),
            str(source),
            "-lm",
        ]
        try:
            compiled = subprocess.run(compile_command, capture_output=True, text=True)
        except OSError as error:
            return [{"name": "compile", "passed": False,
                     "detail": f"cannot run cc: {error}"}]
        if compiled.returncode != 0:
            detail = (compiled.stderr or compiled.stdout).strip()
            return [{"name": "compile", "passed": False,
                     "detail": f"cc failed: {detail}"}]

        results = []
        for test in tests:
            results.append(_run_c_one(program_dir, binary, test, limit))
        return results


def _run_c_one(program_dir, binary, test, limit):
    name = test.get("name", "?")
    with tempfile.TemporaryDirectory() as directory:
        work = pathlib.Path(directory)
        inputs = program_dir / "inputs"
        if inputs.is_dir():
            shutil.copytree(inputs, work, dirs_exist_ok=True)
        for rel, content in test.get("files", {}).items():
            _write_file(work, rel, content)

        kwargs = {"cwd": work, "capture_output": True}
        if "stdin" in test:
            kwargs["input"] = test["stdin"].encode("utf-8")
        else:
            kwargs["stdin"] = subprocess.DEVNULL
        try:
            proc = subprocess.run(
                [str(binary)] + [str(arg) for arg in test.get("args", [])],
                timeout=limit,
                **kwargs,
            )
        except subprocess.TimeoutExpired:
            return {"name": name, "passed": False, "detail": f"timeout after {limit}s"}
        except OSError as error:
            return {"name": name, "passed": False, "detail": f"cannot run: {error}"}

        expected_stdout = test.get("stdout", "")
        actual_stdout = proc.stdout.decode("utf-8", errors="replace")
        if proc.stdout != expected_stdout.encode("utf-8"):
            return {"name": name, "passed": False,
                    "detail": f"stdout: expected {expected_stdout!r}, "
                              f"got {actual_stdout!r}"}
        expected_code = test.get("exit_code", 0)
        if proc.returncode != expected_code:
            stderr = proc.stderr.decode("utf-8", errors="replace")
            detail = f"exit code: expected {expected_code}, got {proc.returncode}"
            if stderr:
                detail += f"; stderr {stderr!r}"
            return {"name": name, "passed": False, "detail": detail}
        for rel, expected in test.get("expect_files", {}).items():
            path = work / rel
            if not path.is_file():
                return {"name": name, "passed": False,
                        "detail": f"missing output file {rel!r}"}
            actual = path.read_bytes()
            if actual != expected.encode("utf-8"):
                return {"name": name, "passed": False,
                        "detail": f"file {rel!r}: expected {expected!r}, "
                                  f"got {actual.decode('utf-8', errors='replace')!r}"}
        return {"name": name, "passed": True, "detail": "ok"}


def verify(corpus_dir, aleph=None):
    """Run every version against tests.json; return a list of failure messages."""
    aleph = aleph or os.environ.get("ALEPH", DEFAULT_ALEPH)
    corpus_dir = pathlib.Path(corpus_dir)
    failures = []
    for program_dir in discover_programs(corpus_dir):
        tests = load_json(program_dir / "tests.json")
        with _suite_task_dir(program_dir) as task_dir:
            for lang in ("aleph", "python", "bash"):
                path = program_dir / LANGUAGE_FILES[lang]
                if not path.is_file():
                    continue
                try:
                    results = suite.run_tests(task_dir, lang, path, aleph=aleph)
                except (OSError, ValueError, KeyError) as error:
                    failures.append(f"{program_dir.name}: {lang}: cannot run: {error}")
                    continue
                for result in results:
                    if not result["passed"]:
                        failures.append(
                            f"{program_dir.name}: {lang}: {result['name']}: "
                            f"{result['detail']}"
                        )

        c_path = program_dir / LANGUAGE_FILES["c"]
        if c_path.is_file():
            for result in run_c_tests(program_dir, c_path, tests):
                if not result["passed"]:
                    failures.append(
                        f"{program_dir.name}: c: {result['name']}: {result['detail']}"
                    )
    return failures


# ---------------------------------------------------------------------------
# Normalisation


def py_norm(source):
    """`ast.unparse` with module/class/function docstrings removed."""
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if isinstance(
            node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Module)
        ):
            body = node.body
            if (
                body
                and isinstance(body[0], ast.Expr)
                and isinstance(body[0].value, ast.Constant)
                and isinstance(body[0].value.value, str)
            ):
                node.body = body[1:] or [ast.Pass()]
    return ast.unparse(tree) + "\n"


def strip_c_comments(source):
    """Remove // and /* */ comments, preserving string/char literals and lines."""
    out = []
    index = 0
    length = len(source)
    state = "code"
    while index < length:
        char = source[index]
        following = source[index + 1] if index + 1 < length else ""
        if state == "code":
            if char == "/" and following == "/":
                state = "line"
                index += 2
            elif char == "/" and following == "*":
                state = "block"
                index += 2
            elif char == '"':
                state = "string"
                out.append(char)
                index += 1
            elif char == "'":
                state = "char"
                out.append(char)
                index += 1
            else:
                out.append(char)
                index += 1
        elif state == "line":
            if char == "\n":
                out.append(char)
                state = "code"
            index += 1
        elif state == "block":
            if char == "*" and following == "/":
                state = "code"
                index += 2
            else:
                if char == "\n":
                    out.append(char)
                index += 1
        else:  # string or char literal
            out.append(char)
            if char == "\\" and index + 1 < length:
                out.append(source[index + 1])
                index += 2
            else:
                if (state == "string" and char == '"') or (
                    state == "char" and char == "'"
                ):
                    state = "code"
                index += 1
    return "".join(out)


def strip_bash_comments(source):
    """Remove shell comments, respecting quotes and ${...} expansions."""
    out = []
    index = 0
    length = len(source)
    quote = None
    while index < length:
        char = source[index]
        if quote:
            out.append(char)
            if quote == '"' and char == "\\" and index + 1 < length:
                out.append(source[index + 1])
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in "'\"":
            quote = char
            out.append(char)
            index += 1
        elif char == "#" and (
            index == 0 or source[index - 1] in " \t\n;|&()"
        ):
            newline = source.find("\n", index)
            index = length if newline == -1 else newline
        elif char == "\\" and index + 1 < length:
            out.append(char)
            out.append(source[index + 1])
            index += 2
        else:
            out.append(char)
            index += 1
    return "".join(out)


def _run_formatter(command, text):
    try:
        proc = subprocess.run(command, input=text, capture_output=True, text=True)
    except OSError:
        return None
    if proc.returncode != 0:
        return None
    return proc.stdout


def maybe_format_c(text):
    """clang-format on stdin when installed; return (text, formatter)."""
    executable = shutil.which("clang-format")
    if executable:
        formatted = _run_formatter([executable], text)
        if formatted is not None:
            return formatted, "clang-format"
    return text, "none"


def maybe_format_bash(text):
    """shfmt on stdin when installed; return (text, formatter)."""
    executable = shutil.which("shfmt")
    if executable:
        formatted = _run_formatter([executable], text)
        if formatted is not None:
            return formatted, "shfmt"
    return text, "none"


def remove_examples(text):
    """Drop Aleph `ex` example lines from formatted source."""
    kept = [
        line for line in text.splitlines() if not line.lstrip().startswith("ex ")
    ]
    return "\n".join(kept) + "\n"


def aleph_fmt(source, aleph):
    """Format Aleph source through `aleph fmt`; return stdout."""
    with tempfile.NamedTemporaryFile(
        "w", suffix=".aph", delete=False, encoding="utf-8"
    ) as handle:
        handle.write(source)
        name = handle.name
    try:
        proc = subprocess.run(
            [aleph, "fmt", name], capture_output=True, text=True
        )
    finally:
        os.unlink(name)
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip()
        raise RuntimeError(f"aleph fmt failed: {detail}")
    return proc.stdout


# ---------------------------------------------------------------------------
# Token counting


def _run_aleph_tokens(aleph, path, vocab=None):
    command = [aleph, "tokens", path, "--tokenizer", "all", "--json"]
    if vocab is not None:
        command.extend(["--vocab", str(vocab)])
    try:
        proc = subprocess.run(command, capture_output=True, text=True)
    except OSError as error:
        raise TokenCountError(f"cannot run {aleph}: {error}") from error
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip() or f"exit {proc.returncode}"
        raise TokenCountError(detail)
    try:
        data = json.loads(proc.stdout)
    except ValueError as error:
        raise TokenCountError(f"invalid JSON from aleph tokens: {error}") from error
    return {
        entry["tokenizer"]: entry.get("count")
        for entry in data.get("tokens", [])
    }


def count_text(text, aleph, vocabs=(), vocab_status=None):
    """Count ``text`` with `aleph tokens --tokenizer all`.

    A vocabulary is attempted separately; when accepted, its ``bpe:*`` counters
    are merged into the result.  When rejected, ``vocab_status`` records why.
    """
    if vocab_status is None:
        vocab_status = {}
    with tempfile.NamedTemporaryFile(
        "w", suffix=".txt", delete=False, encoding="utf-8"
    ) as handle:
        handle.write(text)
        name = handle.name
    try:
        counts = _run_aleph_tokens(aleph, name)
        for vocab in vocabs:
            key = str(vocab)
            try:
                extra = _run_aleph_tokens(aleph, name, vocab)
            except TokenCountError as error:
                vocab_status.setdefault(
                    key, {"available": False, "reason": str(error)}
                )
                continue
            bpe = {
                name_: count
                for name_, count in extra.items()
                if name_.startswith("bpe:")
            }
            if bpe:
                counts.update(bpe)
                vocab_status[key] = {
                    "available": True,
                    "counters": sorted(bpe),
                }
            else:
                vocab_status.setdefault(
                    key,
                    {
                        "available": False,
                        "reason": "aleph tokens returned no bpe counters",
                    },
                )
    finally:
        os.unlink(name)
    return counts


def collect_programs(corpus_dir, aleph, vocabs=(), vocab_status=None):
    """Normalise and count every version in the corpus."""
    if vocab_status is None:
        vocab_status = {}
    programs = []
    for program_dir in discover_programs(corpus_dir):
        meta = load_json(program_dir / "meta.json")
        record = {
            "program": meta.get("program", program_dir.name),
            "group": meta.get("group", "unknown"),
            "category": meta.get("category", "unknown"),
            "source": meta.get("source", {}),
            "languages": [],
            "formatters": {},
            "tokens": {},
        }
        for lang in ("aleph", "python", "c", "bash"):
            path = program_dir / LANGUAGE_FILES[lang]
            if not path.is_file():
                continue
            record["languages"].append(lang)
            source = path.read_text(encoding="utf-8")
            if lang == "aleph":
                formatted = aleph_fmt(source, aleph)
                full_text = formatted
                logic_text = remove_examples(formatted)
                record["formatters"][lang] = "aleph fmt"
            elif lang == "python":
                formatted = py_norm(source)
                full_text = formatted
                logic_text = formatted
                record["formatters"][lang] = "ast.unparse"
            elif lang == "c":
                formatted, formatter = maybe_format_c(strip_c_comments(source))
                full_text = formatted
                logic_text = formatted
                record["formatters"][lang] = formatter
            else:  # bash
                formatted, formatter = maybe_format_bash(
                    strip_bash_comments(source)
                )
                full_text = formatted
                logic_text = formatted
                record["formatters"][lang] = formatter

            full_counts = count_text(full_text, aleph, vocabs, vocab_status)
            if logic_text == full_text:
                logic_counts = full_counts
            else:
                logic_counts = count_text(logic_text, aleph, vocabs, vocab_status)
            names = sorted(set(full_counts) | set(logic_counts))
            record["tokens"][lang] = {
                name: {
                    "full": full_counts.get(name),
                    "logic": logic_counts.get(name),
                }
                for name in names
            }
        programs.append(record)
    return programs


def analysis_tokenizers(programs):
    """`bpe:*` counters when any are available, else ``chars4``."""
    bpe = set()
    has_chars4 = False
    for program in programs:
        for tokens in program["tokens"].values():
            for name in tokens:
                if name.startswith("bpe:"):
                    bpe.add(name)
                elif name == "chars4":
                    has_chars4 = True
    if bpe:
        return sorted(bpe)
    if has_chars4:
        return ["chars4"]
    return []


# ---------------------------------------------------------------------------
# Statistics


def bootstrap_median_ci(values, seed=0, iterations=DEFAULT_BOOTSTRAP):
    """Seeded percentile bootstrap interval for the median."""
    if not values:
        return (None, None)
    generator = random.Random(seed)
    size = len(values)
    medians = []
    for _ in range(iterations):
        sample = [values[generator.randrange(size)] for _ in range(size)]
        medians.append(statistics.median(sample))
    medians.sort()
    low = int(0.025 * iterations)
    high = int(0.975 * iterations)
    return medians[low], medians[min(high, iterations - 1)]


def _exact_wilcoxon_p(ranks, w_plus, total):
    # Ranks can be half-integers (ties); scale by two to stay integral.
    scaled = [int(round(rank * 2)) for rank in ranks]
    target = int(round(w_plus * 2))
    limit = int(round(total * 2))
    counts = [0] * (limit + 1)
    counts[0] = 1
    for rank in scaled:
        for value in range(limit, rank - 1, -1):
            counts[value] += counts[value - rank]
    assignments = 2 ** len(ranks)
    lower = sum(counts[: target + 1])
    upper = sum(counts[target:])
    return min(1.0, 2.0 * min(lower, upper) / assignments)


def _normal_wilcoxon_p(ranks, w_plus, total):
    mean = total / 2.0
    variance = sum(rank * rank for rank in ranks) / 4.0
    if variance <= 0:
        return 1.0
    z = (abs(w_plus - mean) - 0.5) / math.sqrt(variance)
    z = max(0.0, z)
    return min(1.0, math.erfc(z / math.sqrt(2.0)))


def wilcoxon(differences):
    """Two-sided Wilcoxon signed-rank test on paired differences."""
    diffs = [value for value in differences if value != 0]
    n = len(diffs)
    if n == 0:
        return {"statistic": 0.0, "p": 1.0, "method": "exact", "n": 0}

    order = sorted(range(n), key=lambda index: abs(diffs[index]))
    ranks = [0.0] * n
    index = 0
    while index < n:
        end = index
        while (
            end + 1 < n
            and abs(diffs[order[end + 1]]) == abs(diffs[order[index]])
        ):
            end += 1
        average = ((index + 1) + (end + 1)) / 2.0
        for position in range(index, end + 1):
            ranks[order[position]] = average
        index = end + 1

    w_plus = sum(ranks[i] for i in range(n) if diffs[i] > 0)
    total = sum(ranks)
    if n <= EXACT_WILCOXON_LIMIT:
        p = _exact_wilcoxon_p(ranks, w_plus, total)
        method = "exact"
    else:
        p = _normal_wilcoxon_p(ranks, w_plus, total)
        method = "normal"
    return {"statistic": w_plus, "p": p, "method": method, "n": n}


def ratio_stats(programs, tokenizer, with_examples, seed=0,
                iterations=DEFAULT_BOOTSTRAP):
    """Median Aleph/Python ratio, bootstrap CI and Wilcoxon on differences."""
    ratios = []
    differences = []
    for program in programs:
        aleph = program["tokens"].get("aleph", {}).get(tokenizer)
        python = program["tokens"].get("python", {}).get(tokenizer)
        if not aleph or not python:
            continue
        aleph_count = aleph["full" if with_examples else "logic"]
        python_count = python["full"]
        if aleph_count is None or python_count in (None, 0):
            continue
        ratios.append(aleph_count / python_count)
        differences.append(aleph_count - python_count)
    if not ratios:
        return {"n": 0, "median": None, "ci95": None, "wilcoxon": None}
    low, high = bootstrap_median_ci(ratios, seed, iterations)
    return {
        "n": len(ratios),
        "median": statistics.median(ratios),
        "ci95": [low, high],
        "wilcoxon": wilcoxon(differences),
    }


def break_even(llms_tokens, python_tokens, aleph_tokens):
    """Programs needed to repay loading llms.txt; ``None`` when Aleph is not shorter."""
    if llms_tokens is None:
        return None
    saving = python_tokens - aleph_tokens
    if saving <= 0:
        return None
    return llms_tokens / saving


def _break_even_for(programs, tokenizer, llms_tokens):
    python_total = 0
    aleph_total = 0
    seen = False
    for program in programs:
        aleph = program["tokens"].get("aleph", {}).get(tokenizer)
        python = program["tokens"].get("python", {}).get(tokenizer)
        if not aleph or not python:
            continue
        if aleph.get("logic") is None or python.get("logic") is None:
            continue
        aleph_total += aleph["logic"]
        python_total += python["logic"]
        seen = True
    if not seen:
        return None
    return break_even(llms_tokens, python_total, aleph_total)


def compute_report(programs, tokenizers, llms_counts, vocab_status=None,
                   seed=0, iterations=DEFAULT_BOOTSTRAP):
    """Assemble the JSON report from already-counted program records."""
    vocab_status = vocab_status or {}
    groups = sorted({program["group"] for program in programs})
    categories = sorted({program["category"] for program in programs})
    coverage = {
        lang: sum(1 for program in programs if lang in program["languages"])
        for lang in LANGS
    }
    coverage["programs"] = len(programs)

    report = {
        "seed": seed,
        "bootstrap_iterations": iterations,
        "tokenizers": [
            {"name": name, "estimate": name == "chars4"}
            for name in tokenizers
        ],
        "vocabularies": vocab_status,
        "coverage": coverage,
        "programs": programs,
        "ratios": {},
        "groups": {},
        "categories": {},
        "break_even": {},
        "llms_tokens": {
            name: llms_counts.get(name) for name in tokenizers
        },
    }

    for tokenizer in tokenizers:
        report["ratios"][tokenizer] = {
            "with_examples": ratio_stats(
                programs, tokenizer, True, seed, iterations
            ),
            "without_examples": ratio_stats(
                programs, tokenizer, False, seed, iterations
            ),
        }
        report["groups"][tokenizer] = {}
        for group in groups:
            subset = [
                program for program in programs if program["group"] == group
            ]
            report["groups"][tokenizer][group] = {
                "with_examples": ratio_stats(
                    subset, tokenizer, True, seed, iterations
                ),
                "without_examples": ratio_stats(
                    subset, tokenizer, False, seed, iterations
                ),
            }
        report["categories"][tokenizer] = {}
        for category in categories:
            subset = [
                program
                for program in programs
                if program["category"] == category
            ]
            report["categories"][tokenizer][category] = {
                "with_examples": ratio_stats(
                    subset, tokenizer, True, seed, iterations
                ),
                "without_examples": ratio_stats(
                    subset, tokenizer, False, seed, iterations
                ),
            }
        report["break_even"][tokenizer] = {}
        for group in groups + ["all"]:
            subset = (
                programs
                if group == "all"
                else [
                    program
                    for program in programs
                    if program["group"] == group
                ]
            )
            value = _break_even_for(
                subset, tokenizer, llms_counts.get(tokenizer)
            )
            report["break_even"][tokenizer][group] = (
                "never" if value is None else value
            )
    return report


def build_report(corpus_dir, vocabs=(), seed=0, aleph=None,
                 iterations=DEFAULT_BOOTSTRAP):
    """Normalise, count and summarise a corpus."""
    aleph = aleph or os.environ.get("ALEPH", DEFAULT_ALEPH)
    vocab_status = {}
    programs = collect_programs(
        corpus_dir, aleph, vocabs=vocabs, vocab_status=vocab_status
    )
    tokenizers = analysis_tokenizers(programs)
    llms_path = REPO / "llms.txt"
    llms_counts = {}
    if tokenizers and llms_path.is_file():
        llms_counts = count_text(
            llms_path.read_text(encoding="utf-8"),
            aleph,
            vocabs,
            vocab_status,
        )
    return compute_report(
        programs,
        tokenizers,
        llms_counts,
        vocab_status=vocab_status,
        seed=seed,
        iterations=iterations,
    )


# ---------------------------------------------------------------------------
# Printing


def _stat_line(label, stats):
    if not stats or stats.get("n", 0) == 0:
        return f"  {label}: n=0"
    wilcoxon_stats = stats["wilcoxon"]
    low, high = stats["ci95"]
    return (
        f"  {label}: n={stats['n']} median={stats['median']:.3f} "
        f"ci95=[{low:.3f}, {high:.3f}] "
        f"wilcoxon p={wilcoxon_stats['p']:.4g} "
        f"({wilcoxon_stats['method']}, n={wilcoxon_stats['n']})"
    )


def _cell(value):
    return "-" if value is None else str(value)


def _token_table(report, tokenizer):
    lines = [
        f"{'program':<26} {'group':<14} {'category':<12} "
        f"{'aleph':>6} {'+ex':>6} {'python':>7} {'c':>6} {'bash':>6} {'a/p':>7}"
    ]
    for program in report["programs"]:
        aleph = program["tokens"].get("aleph", {}).get(tokenizer)
        python = program["tokens"].get("python", {}).get(tokenizer)
        c_tokens = program["tokens"].get("c", {}).get(tokenizer)
        bash = program["tokens"].get("bash", {}).get(tokenizer)
        logic = aleph["logic"] if aleph else None
        full = aleph["full"] if aleph else None
        python_count = python["logic"] if python else None
        c_count = c_tokens["logic"] if c_tokens else None
        bash_count = bash["logic"] if bash else None
        ratio = (
            logic / python_count
            if logic is not None and python_count
            else None
        )
        ratio_text = f"{ratio:.3f}" if ratio is not None else "-"
        lines.append(
            f"{program['program']:<26} {program['group']:<14} "
            f"{program['category']:<12} {_cell(logic):>6} {_cell(full):>6} "
            f"{_cell(python_count):>7} {_cell(c_count):>6} "
            f"{_cell(bash_count):>6} {ratio_text:>7}"
        )
    return "\n".join(lines)


def print_report(report):
    coverage = report["coverage"]
    print(f"Corpus: {coverage['programs']} programs")
    print(
        "Coverage: "
        + ", ".join(
            f"{lang}={coverage[lang]}" for lang in LANGS
        )
    )
    formatters = sorted(
        {
            (formatter, lang)
            for program in report["programs"]
            for lang, formatter in program["formatters"].items()
        }
    )
    if formatters:
        print(
            "Formatters: "
            + ", ".join(f"{lang}={formatter}" for formatter, lang in formatters)
        )

    if report["vocabularies"]:
        print("Vocabularies:")
        for path, status in sorted(report["vocabularies"].items()):
            if status.get("available"):
                counters = ", ".join(status.get("counters", []))
                print(f"  {path}: available ({counters})")
            else:
                print(f"  {path}: unavailable: {status.get('reason', '?')}")

    for entry in report["tokenizers"]:
        name = entry["name"]
        label = f"{name} (estimate)" if entry["estimate"] else name
        print(f"\nTokens per program ({label})")
        print(_token_table(report, name))
        print(
            f"llms.txt tokens: {_cell(report['llms_tokens'].get(name))}"
        )

        print(f"\nRatios ({label})")
        print(_stat_line("overall with examples",
                         report["ratios"][name]["with_examples"]))
        print(_stat_line("overall without examples",
                         report["ratios"][name]["without_examples"]))
        for group in sorted(report["groups"][name]):
            print(_stat_line(
                f"group {group} with examples",
                report["groups"][name][group]["with_examples"],
            ))
            print(_stat_line(
                f"group {group} without examples",
                report["groups"][name][group]["without_examples"],
            ))
        for category in sorted(report["categories"][name]):
            print(_stat_line(
                f"category {category} with examples",
                report["categories"][name][category]["with_examples"],
            ))
            print(_stat_line(
                f"category {category} without examples",
                report["categories"][name][category]["without_examples"],
            ))

        print("\nBreak-even N = tokens(llms.txt) / "
              "(tokens(Python) - tokens(Aleph)), without examples:")
        parts = []
        for group, value in report["break_even"][name].items():
            if value is None or value == "never":
                parts.append(f"{group}=never")
            else:
                parts.append(f"{group}={value:.1f}")
        print("  " + ", ".join(parts))


# ---------------------------------------------------------------------------
# CLI


def _cmd_verify(args):
    failures = verify(args.corpus_dir)
    for failure in failures:
        print(failure)
    if failures:
        print(f"FAIL: {len(failures)} failure(s)", file=sys.stderr)
        return 1
    print("OK: every version passed")
    return 0


def _cmd_report(args):
    report = build_report(
        args.corpus_dir,
        vocabs=args.vocab,
        seed=args.seed,
    )
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print_report(report)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    verify_parser = sub.add_parser("verify", help="run every version against tests.json")
    verify_parser.add_argument("corpus_dir")
    verify_parser.set_defaults(func=_cmd_verify)

    report_parser = sub.add_parser("report", help="count tokens and summarise")
    report_parser.add_argument("corpus_dir")
    report_parser.add_argument(
        "--vocab", action="append", default=[],
        help="local tokenizer vocabulary file (repeatable)",
    )
    report_parser.add_argument("--json", action="store_true")
    report_parser.add_argument("--seed", type=int, default=0)
    report_parser.set_defaults(func=_cmd_report)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
