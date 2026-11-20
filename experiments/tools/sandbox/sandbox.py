#!/usr/bin/env python3
"""File-access sandbox for EXP-002: run a program and log what it touches.

Usage:
  sandbox.py run --lang {python|bash} [--cwd DIR] [--timeout SECONDS] PROGRAM [args...]

The sandbox only observes: it never blocks an access or changes the program.
Python programs run through pyaudit.py (sys.addaudithook); bash programs run
under strace when it is installed, otherwise the run is marked not observed.
Python 3 standard library only, offline. See README.md.
"""
import argparse
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
PYAUDIT = HERE / "pyaudit.py"
LOG_ENV = "ALEPH_SANDBOX_LOG"
DEFAULT_TIMEOUT = 10
LANGS = ("python", "bash")
STRACE_SYSCALLS = "open,openat,creat,unlink,unlinkat,rename,renameat,mkdir,rmdir"
TRACED_SYSCALLS = frozenset(STRACE_SYSCALLS.split(","))

# System files that every process touches while starting; documented in README.
SYSTEM_PATHS = (
    "/lib",
    "/lib64",
    "/usr/lib",
    "/usr/share/locale",
    "/proc/self",
    "/dev/null",
    "/dev/tty",
)
SYSTEM_PREFIXES = (
    "/etc/ld.so",
)


def strace_path():
    """The strace executable on PATH, or None."""
    return shutil.which("strace")


def bash_path():
    """bash on PATH, falling back to the usual absolute locations."""
    found = shutil.which("bash")
    if found:
        return found
    for candidate in ("/bin/bash", "/usr/bin/bash"):
        if os.path.isfile(candidate):
            return candidate
    return "bash"


def _normalize(path, base):
    """Absolute normalized path relative to base, or None for non-paths."""
    if path is None:
        return None
    try:
        path = os.fspath(path)
    except TypeError:
        return None
    if isinstance(path, bytes):
        path = os.fsdecode(path)
    if not isinstance(path, str) or not path:
        return None
    if not os.path.isabs(path):
        path = os.path.join(str(base), path)
    return os.path.normpath(os.path.abspath(path))


def _is_system_path(path):
    for base in SYSTEM_PATHS:
        if path == base or path.startswith(base + "/"):
            return True
    for prefix in SYSTEM_PREFIXES:
        if path == prefix or path.startswith(prefix):
            return True
    return False


def _run(command, cwd, stdin, timeout, env=None):
    """subprocess.run, but a timeout kills the whole process group (strace -f tracees too)."""
    data = stdin.encode("utf-8") if isinstance(stdin, str) else stdin
    proc = subprocess.Popen(command, cwd=str(cwd), env=env, start_new_session=True,
                            stdin=subprocess.PIPE if data is not None else subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        out, err = proc.communicate(data, timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        proc.communicate()
        raise
    return subprocess.CompletedProcess(command, proc.returncode, out, err)


def _parse_python_log(log_path, cwd, program):
    if not log_path.is_file():
        return []
    program_real = os.path.realpath(program)
    accesses = []
    for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except ValueError:
            continue
        path = _normalize(record.get("path"), cwd)
        if path is None or _is_system_path(path) or os.path.realpath(path) == program_real:
            continue
        accesses.append({
            "op": record.get("op", "other"),
            "path": path,
            "ok": bool(record.get("ok", True)),
        })
    return accesses


def _c_strings(text):
    """The C string literal arguments in a strace line, unescaped."""
    result = []
    index = 0
    simple = {"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\", "0": "\0"}
    while index < len(text):
        if text[index] != '"':
            index += 1
            continue
        index += 1
        chars = []
        while index < len(text):
            char = text[index]
            if char == "\\" and index + 1 < len(text):
                nxt = text[index + 1]
                if nxt in simple:
                    chars.append(simple[nxt])
                    index += 2
                elif nxt.isdigit():
                    digits = ""
                    index += 1
                    while index < len(text) and len(digits) < 3 and text[index].isdigit():
                        digits += text[index]
                        index += 1
                    try:
                        chars.append(chr(int(digits, 8)))
                    except ValueError:
                        chars.append("\\" + digits)
                else:
                    chars.append(nxt)
                    index += 2
            elif char == '"':
                index += 1
                break
            else:
                chars.append(char)
                index += 1
        result.append("".join(chars))
    return result


def _strace_ok(line):
    before, separator, tail = line.rpartition(" = ")
    if not separator:
        return True
    return not tail.strip().startswith("-")


def _open_flags_write(line):
    return any(flag in line for flag in
               ("O_WRONLY", "O_RDWR", "O_CREAT", "O_TRUNC", "O_APPEND"))


def _parse_strace_log(log_path, cwd, program):
    if not log_path.is_file():
        return []
    program_real = os.path.realpath(program)
    accesses = []

    def add(path, op, ok):
        normalized = _normalize(path, cwd)
        if normalized is None:
            return
        if _is_system_path(normalized) or os.path.realpath(normalized) == program_real:
            return
        accesses.append({"op": op, "path": normalized, "ok": ok})

    for raw in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = re.sub(r"^\d+\s+", "", raw.strip())
        if "(" not in line or "<unfinished" in line or "resumed>" in line:
            continue
        name = line.split("(", 1)[0]
        if name not in TRACED_SYSCALLS:
            continue
        paths = _c_strings(line)
        if not paths:
            continue
        ok = _strace_ok(line)
        if name in ("open", "openat", "creat"):
            op = "write" if name == "creat" or _open_flags_write(line) else "read"
            add(paths[0], op, ok)
        elif name in ("unlink", "unlinkat", "mkdir", "rmdir"):
            add(paths[0], "write", ok)
        elif name in ("rename", "renameat"):
            add(paths[0], "write", ok)
            if len(paths) > 1:
                add(paths[1], "write", ok)
    return accesses


def run_logged(lang, program, args=None, cwd=None, stdin=None, timeout=DEFAULT_TIMEOUT):
    """Run PROGRAM and return its output plus the file accesses it attempted.

    The result is {returncode, stdout, stderr, accesses, observed}: stdout and
    stderr are text, accesses is a list of {op, path, ok} with absolute paths
    and observed is False only when bash runs without strace.
    """
    if lang not in LANGS:
        raise ValueError(f"unknown language {lang!r}; expected one of {sorted(LANGS)}")
    arguments = [str(argument) for argument in (args or ())]
    base = pathlib.Path(cwd).resolve() if cwd else pathlib.Path.cwd()
    # A relative program is relative to the caller, like a shell command line;
    # cwd is the working directory the program itself runs in.
    program = _normalize(program, pathlib.Path.cwd())
    if program is None:
        raise ValueError("program path is required")

    with tempfile.TemporaryDirectory() as directory:
        log_path = pathlib.Path(directory) / "access.log"
        observed = True
        accesses = []
        if lang == "python":
            env = os.environ.copy()
            env[LOG_ENV] = str(log_path)
            command = [sys.executable, str(PYAUDIT), program] + arguments
            proc = _run(command, base, stdin, timeout, env=env)
            accesses = _parse_python_log(log_path, base, program)
        else:
            strace = strace_path()
            if strace:
                command = [
                    strace, "-f", "-qq", "-s", "4096",
                    "-e", "trace=" + STRACE_SYSCALLS,
                    "-o", str(log_path), bash_path(), program,
                ] + arguments
                proc = _run(command, base, stdin, timeout)
                accesses = _parse_strace_log(log_path, base, program)
            else:
                observed = False
                proc = _run([bash_path(), program] + arguments, base, stdin, timeout)

    return {
        "returncode": proc.returncode,
        "stdout": proc.stdout.decode("utf-8", errors="replace"),
        "stderr": proc.stderr.decode("utf-8", errors="replace"),
        "accesses": accesses,
        "observed": observed,
    }


def outside(accesses, declared_read, declared_write, run_dir):
    """Accesses that the task's declared files do not cover.

    A declared write file may be read (the program produced it earlier).
    Reading or listing anything undeclared counts, including paths inside the
    run directory; system paths from the documented ignore list do not.
    """
    base = pathlib.Path(run_dir).resolve()
    readable = {_normalize(path, base) for path in (declared_read or ())}
    writable = {_normalize(path, base) for path in (declared_write or ())}
    readable.discard(None)
    writable.discard(None)
    result = []
    for access in accesses or ():
        path = _normalize(access.get("path"), base)
        if path is None or _is_system_path(path):
            continue
        op = access.get("op", "other")
        if op == "read":
            if path in readable or path in writable:
                continue
        elif op == "write":
            if path in writable:
                continue
        elif path in readable or path in writable:
            continue
        item = dict(access)
        item["path"] = path
        result.append(item)
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description="Log file accesses of a program")
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="run one program and print JSON")
    run.add_argument("--lang", required=True, choices=LANGS)
    run.add_argument("--cwd", default=None)
    run.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT)
    run.add_argument("program")
    run.add_argument("args", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if args.command == "run":
        result = run_logged(args.lang, args.program, args.args, args.cwd,
                            None, args.timeout)
        print(json.dumps(result, indent=2))
        return 0
    return 2  # pragma: no cover - argparse requires a subcommand


if __name__ == "__main__":
    sys.exit(main())
