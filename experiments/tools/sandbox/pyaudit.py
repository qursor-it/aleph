#!/usr/bin/env python3
"""Run a Python program with an audit hook that logs file accesses.

Usage:
  ALEPH_SANDBOX_LOG=LOG python3 pyaudit.py PROGRAM [args...]

The log is one JSON object per line: {"op", "path", "ok"}. The hook only
records; it never blocks or changes an operation. See sandbox.py and README.md.
"""
import json
import os
import pkgutil  # noqa: F401 - preloaded so runpy does not scan sys.path
import runpy
import sys
import traceback

LOG_ENV = "ALEPH_SANDBOX_LOG"
WRITE_FLAGS = os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND


def _normalize(path):
    """Absolute normalized path, or None when the value is not a path."""
    try:
        path = os.fspath(path)
    except TypeError:
        return None
    if isinstance(path, bytes):
        path = os.fsdecode(path)
    if not isinstance(path, str) or not path:
        return None
    if not os.path.isabs(path):
        path = os.path.join(os.getcwd(), path)
    return os.path.normpath(os.path.abspath(path))


def _open_op(mode, flags):
    if isinstance(mode, str) and mode:
        return "write" if any(char in mode for char in "wax+") else "read"
    if isinstance(flags, int) and flags & WRITE_FLAGS:
        return "write"
    return "read"


def _interpreter_prefixes():
    """Directories of the interpreter, its stdlib and installed packages."""
    prefixes = set()
    for candidate in (sys.prefix, sys.base_prefix, os.path.dirname(os.__file__)):
        if candidate:
            prefixes.add(os.path.realpath(candidate))
    return prefixes


def run(program, program_args):
    program = os.path.abspath(program)
    log_path = os.environ.get(LOG_ENV)
    # Opened before the hook is installed, so the log itself is never logged.
    log_handle = None
    if log_path:
        try:
            log_handle = open(log_path, "w", encoding="utf-8")
        except OSError:
            log_handle = None

    prefixes = _interpreter_prefixes()
    program_real = os.path.realpath(program)
    wrapper_real = os.path.realpath(__file__)
    program_dir = os.path.dirname(program)
    run_dir = os.getcwd()
    started = [False]

    def ignored(path):
        real = os.path.realpath(path)
        if real in (program_real, wrapper_real):
            return True
        return any(real == prefix or real.startswith(prefix + os.sep)
                   for prefix in prefixes)

    def record(op, path, ok=True):
        normalized = _normalize(path)
        if normalized is None or ignored(normalized):
            return
        event = {"op": op, "path": normalized, "ok": bool(ok)}
        if log_handle is not None:
            try:
                log_handle.write(json.dumps(event) + "\n")
                log_handle.flush()
            except OSError:
                pass

    def hook(event, args):
        if not started[0]:
            return
        try:
            if event == "open":
                mode = args[1] if len(args) > 1 else None
                flags = args[2] if len(args) > 2 else None
                record(_open_op(mode, flags), args[0])
            elif event in ("os.remove", "os.rmdir", "os.mkdir", "os.truncate"):
                record("write", args[0])
            elif event == "os.rename":
                record("write", args[0])
                record("write", args[1])
            elif event in ("os.listdir", "os.scandir"):
                # Import machinery scans the script directory; it is not the
                # program listing its own files when the script lives elsewhere.
                listed = _normalize(args[0])
                if listed == program_dir and program_dir != run_dir:
                    return
                record("other", args[0])
            elif event == "shutil.copyfile":
                record("read", args[0])
                record("write", args[1])
            elif event in ("shutil.copymode", "shutil.copystat"):
                record("read", args[0])
                record("write", args[1])
            elif event == "shutil.copytree":
                record("read", args[0])
                record("write", args[1])
            elif event == "shutil.move":
                record("write", args[0])
                record("write", args[1])
            elif event == "shutil.rmtree":
                record("write", args[0])
        except Exception:
            # An audit hook must never raise: that would change the program.
            pass

    sys.addaudithook(hook)
    sys.argv = [program] + list(program_args)
    if sys.path:
        sys.path[0] = program_dir
    else:
        sys.path.append(program_dir)

    started[0] = True
    try:
        runpy.run_path(program, run_name="__main__")
        return 0
    except SystemExit as exc:
        value = exc.code
        if value is None:
            return 0
        if isinstance(value, int):
            return value
        sys.stderr.write(str(value) + "\n")
        return 1
    except BaseException:
        traceback.print_exc()
        return 1
    finally:
        if log_handle is not None:
            try:
                log_handle.close()
            except OSError:
                pass


def main(argv):
    if len(argv) < 2:
        sys.stderr.write("usage: pyaudit.py PROGRAM [args...]\n")
        return 2
    return run(argv[1], argv[2:])


if __name__ == "__main__":
    sys.exit(main(sys.argv))
