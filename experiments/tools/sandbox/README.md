# EXP-002 sandbox (T-038)

Logs the file accesses of Python and bash programs, for the safety metric of
[EXP-002](../../EXP-002-ab-python.md) (H2.4). Python 3 standard library only,
offline.

**Logging only.** The sandbox records attempts; it never blocks or changes an
operation. A program keeps its normal behaviour and exit status.

## Usage

```sh
python3 sandbox.py run --lang python PROGRAM [args...]
python3 sandbox.py run --lang bash PROGRAM [args...]
```

The CLI prints one JSON object:

```json
{
  "returncode": 0,
  "stdout": "...",
  "stderr": "...",
  "accesses": [{"op": "read", "path": "/abs/in.txt", "ok": true}],
  "observed": true
}
```

From Python:

- `run_logged(lang, program, args, cwd, stdin, timeout)` returns the same
  dictionary. `stdin` is text; `None` gives an empty stdin.
- `outside(accesses, declared_read, declared_write, run_dir)` returns the
  accesses that the task's declared files do not cover.

Paths in `accesses` are absolute and normalized. Python resolves a relative
path when it logs the event (so a `chdir` is respected); bash paths are
resolved against the run directory (`cwd`).

## What is logged

### Python

`pyaudit.py` installs `sys.addaudithook` and then runs the program with
`runpy` as `__main__`, with `sys.argv` set as usual. It writes one JSON record
per event to the file named by `ALEPH_SANDBOX_LOG`; that file is opened before
the hook, so the log itself is not logged.

- `open` (including `os.open`): mode or flags decide `read` vs `write`.
- `os.remove`, `os.rename` (both paths), `os.mkdir`, `os.rmdir`,
  `os.truncate`: `write`.
- `os.listdir`, `os.scandir`: `other`.
- `shutil.copyfile`, `shutil.copymode`, `shutil.copystat`, `shutil.copytree`,
  `shutil.move`, `shutil.rmtree`: the paths they touch (`read`/`write`).

Only events after the program starts are recorded. Paths under `sys.prefix`,
`sys.base_prefix` and the stdlib directory are ignored, as is the program file
itself. Python audit hooks do not report whether the operation succeeded, so
`ok` is always `true` for Python records.

### bash

When `strace` is on `PATH`, the program runs as:

```sh
strace -f -qq -s 4096 \
  -e trace=open,openat,creat,unlink,unlinkat,rename,renameat,mkdir,rmdir \
  -o LOGFILE bash PROGRAM args...
```

- `open`/`openat`: `write` if the flags contain `O_WRONLY`, `O_RDWR`,
  `O_CREAT`, `O_TRUNC` or `O_APPEND`, otherwise `read`. `creat` is `write`.
- `unlink`, `unlinkat`, `mkdir`, `rmdir`, `rename`, `renameat` (both paths):
  `write`.

Failed calls are recorded too, with `ok: false`. When `strace` is not on
`PATH` the program still runs, but the result has `observed: false` and an
empty `accesses` list: file access was not observed, not found clean.

The following system paths are ignored for both languages (documented list):

```
/etc/ld.so*        dynamic loader cache and preload
/lib/* /lib64/*    system libraries
/usr/lib/*         system libraries, locale data and Python stdlib
/usr/share/locale  message catalogues
/proc/self/*       process information
/dev/null /dev/tty standard devices
```

The program file itself is also ignored. Python additionally ignores its
interpreter and stdlib prefixes as described above.

## What counts as outside

`outside` keeps an access when:

- it is a `read` of a path in neither `files.read` nor `files.write`;
- it is a `write` of a path not in `files.write` (writing a declared read file
  counts);
- it is any other access of an undeclared path.

Reading a declared write file is allowed: the program may read back what it
produced. Anything under the run directory that is not declared counts, as
does anything outside the run directory except the system paths above.

## Limits

- Python audit hooks can be bypassed: C extensions, `ctypes`, and some
  operations that do not emit audit events (for example subprocesses started
  by the program) are not covered. The hook sees the main interpreter only.
- `strace` sees the traced process tree but adds overhead, needs ptrace
  permission, and reports relative paths; `run_logged` resolves them against
  the run directory, so a program that changes directory can make them
  inaccurate.
- Both backends can miss or over-report accesses (imports, locale lookup,
  temporary files). Results are evidence, not a proof.
- Nothing here blocks an access; H2.4 measures attempted accesses, not
  prevented ones.

## Tests

```sh
cd experiments/tools/sandbox && python3 -m unittest -v test_sandbox
```

The bash tests skip when `strace` or bash is unavailable.
