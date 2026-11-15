#!/usr/bin/env bash
# T-021 benchmark harness.
#
# For every task in bench/tasks.md, compare four implementations against the
# Aleph golden output: native Aleph (`aleph build`), the interpreter
# (`aleph run`), the Python reference and the C reference. Then time each one
# and print a markdown table.
#
# Env:
#   ALEPH        path to an existing `aleph` binary (skips the release build)
#   BENCH_RUNS   timed runs per engine (default 3)
#   BENCH_TASKS  whitespace-separated task subset, e.g. BENCH_TASKS="fib primes"
#   BENCH_SCALE  non-zero adds the bench-only `fib35` row (default: no)
#   CC           C compiler (default cc; must be a bare executable)
#
# Output is compared before timing; any mismatch exits non-zero. Wall time is
# measured with bash EPOCHREALTIME (microseconds); /usr/bin/time is used only
# for peak RSS.
set -euo pipefail
export LC_ALL=C

: "${EPOCHREALTIME:?bash 5.0 or newer is required}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APH_DIR="$ROOT/tests/aph"
REF_DIR="$ROOT/bench/ref"
HEAVY_DIR="$ROOT/bench/heavy"
EXPECT_DIR="$APH_DIR/expect"

export PATH="$HOME/.cargo/bin:$PATH"

NORMAL_TASKS=(
    fib total evens primes fizzbuzz palindrome wordcount linecount maxmin
    points reverse gcd upper histogram collatz
)
HEAVY_TASKS=(fib35)
KNOWN_TASKS=("${NORMAL_TASKS[@]}" "${HEAVY_TASKS[@]}")

TASKS=("${NORMAL_TASKS[@]}")
if [[ -n "${BENCH_SCALE:-}" && "${BENCH_SCALE}" != "0" ]]; then
    TASKS+=("${HEAVY_TASKS[@]}")
fi
if [[ -n "${BENCH_TASKS:-}" ]]; then
    read -r -a TASKS <<<"$BENCH_TASKS"
fi

BENCH_RUNS="${BENCH_RUNS:-3}"
if ! [[ "$BENCH_RUNS" =~ ^[0-9]+$ ]] || ((BENCH_RUNS < 1)); then
    echo "error: BENCH_RUNS must be a positive integer" >&2
    exit 2
fi

for task in "${TASKS[@]}"; do
    if [[ " ${KNOWN_TASKS[*]} " != *" $task "* ]]; then
        echo "error: unknown benchmark task: $task" >&2
        exit 2
    fi
done

if [[ -z "${ALEPH:-}" ]]; then
    echo "building target/release/aleph (CARGO_BUILD_JOBS=1) ..." >&2
    (
        cd "$ROOT"
        nice -n 15 env CARGO_BUILD_JOBS=1 cargo build --release
    )
    ALEPH="$ROOT/target/release/aleph"
fi
if [[ "$ALEPH" != /* ]]; then
    ALEPH="$PWD/$ALEPH"
fi
if [[ ! -x "$ALEPH" ]]; then
    echo "error: ALEPH is not an executable file: $ALEPH" >&2
    exit 2
fi

CC_BIN="${CC:-cc}"
if ! command -v "$CC_BIN" >/dev/null 2>&1; then
    echo "error: C compiler not found: $CC_BIN" >&2
    exit 2
fi

PYTHON=""
if command -v python3 >/dev/null 2>&1; then
    PYTHON="$(command -v python3)"
else
    echo "warning: python3 not found; the python columns will be n/a" >&2
fi

if [[ -x /usr/bin/time ]]; then
    TIME_BIN=/usr/bin/time
else
    TIME_BIN=""
    echo "warning: /usr/bin/time not found; rss columns will be n/a" >&2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/aleph-bench.XXXXXX")"
cleanup() {
    rm -rf "$WORK"
}
trap cleanup EXIT

# The `.out`/`.err`/`.exit`/`.args`/`.files` prefix for a task.
golden_prefix() {
    case "$1" in
        fib35) printf '%s\n' "$HEAVY_DIR/fib35" ;;
        *) printf '%s\n' "$APH_DIR/bench_$1" ;;
    esac
}

median_ms() {
    local file=$1
    if [[ ! -s "$file" ]]; then
        printf 'n/a\n'
        return
    fi
    sort -n "$file" | awk '
        { value[NR] = $1 }
        END {
            if (NR % 2 == 1) {
                printf "%.2f\n", value[(NR + 1) / 2]
            } else {
                printf "%.2f\n", (value[NR / 2] + value[NR / 2 + 1]) / 2
            }
        }'
}

clear_expected_files() {
    local file
    for file in "${EXPECTED_FILES[@]}"; do
        rm -f "$workdir/$file"
    done
}

run_with_args() {
    local out=$1
    local err=$2
    local status
    shift 2
    set +e
    (
        cd "$workdir"
        if (( ${#ARGS[@]} > 0 )); then
            "$@" "${ARGS[@]}"
        else
            "$@"
        fi
    ) >"$out" 2>"$err"
    status=$?
    set -e
    return "$status"
}

fail_mismatch() {
    echo "MISMATCH: task $task, $1: $2" >&2
    exit 1
}

verify_engine() {
    local label=$1
    local status=0
    shift

    clear_expected_files
    run_with_args "$workdir/verify.out" "$workdir/verify.err" "$@" || status=$?

    if ! cmp -s "$golden_prefix.out" "$workdir/verify.out"; then
        diff -u "$golden_prefix.out" "$workdir/verify.out" >&2 || true
        fail_mismatch "$label" "stdout differs from $(basename "$golden_prefix.out")"
    fi

    if [[ -f "$golden_prefix.err" ]]; then
        if ! cmp -s "$golden_prefix.err" "$workdir/verify.err"; then
            fail_mismatch "$label" "stderr differs from $(basename "$golden_prefix.err")"
        fi
    elif [[ -s "$workdir/verify.err" ]]; then
        fail_mismatch "$label" "unexpected stderr"
    fi

    if (( status != expected_status )); then
        fail_mismatch "$label" "exit $status, expected $expected_status"
    fi

    local file
    for file in "${EXPECTED_FILES[@]}"; do
        if ! cmp -s "$EXPECT_DIR/$file" "$workdir/$file"; then
            fail_mismatch "$label" "written file $file differs from tests/aph/expect/$file"
        fi
    done
}

# Time one engine BENCH_RUNS times around the process itself, in microseconds.
time_engine() {
    local times=$1
    local want_status=$2
    shift 2

    local command=("$@")
    if (( ${#ARGS[@]} > 0 )); then
        command+=("${ARGS[@]}")
    fi

    local run
    for ((run = 0; run < BENCH_RUNS; run += 1)); do
        clear_expected_files
        rm -f "$workdir/timing.txt"
        local start end code status
        set +e
        (
            cd "$workdir"
            start=$EPOCHREALTIME
            "${command[@]}" >"$workdir/time.out" 2>"$workdir/time.err"
            code=$?
            end=$EPOCHREALTIME
            printf '%s %s %s\n' "$start" "$end" "$code" >"$workdir/timing.txt"
            exit "$code"
        )
        status=$?
        set -e
        if [[ ! -s "$workdir/timing.txt" ]]; then
            echo "error: task $task: timed run produced no timing data" >&2
            exit 1
        fi
        read -r start end code <"$workdir/timing.txt"
        if (( status != want_status || code != want_status )); then
            cat "$workdir/time.err" >&2
            fail_mismatch "timed run" "exit $code, expected $want_status"
        fi
        awk -v s="$start" -v e="$end" \
            'BEGIN { printf "%.6f\n", (e - s) * 1000 }' >>"$times"
    done
}

# Run one engine once under /usr/bin/time just to read its peak RSS (KB).
rss_engine() {
    local rss_file=$1
    local want_status=$2
    shift 2

    local command=("$@")
    if (( ${#ARGS[@]} > 0 )); then
        command+=("${ARGS[@]}")
    fi

    if [[ -z "$TIME_BIN" ]]; then
        printf 'n/a\n' >"$rss_file"
        return
    fi

    clear_expected_files
    rm -f "$workdir/rss.txt"
    local status
    set +e
    (
        cd "$workdir"
        "$TIME_BIN" -f "%M" -o "$workdir/rss.txt" "${command[@]}" \
            >"$workdir/rss.out" 2>"$workdir/rss.err"
    )
    status=$?
    set -e
    if (( status != want_status )); then
        cat "$workdir/rss.err" >&2
        fail_mismatch "rss run" "exit $status, expected $want_status"
    fi
    local rss
    if [[ -s "$workdir/rss.txt" ]]; then
        read -r rss <"$workdir/rss.txt"
        printf '%s\n' "$rss" >"$rss_file"
    else
        printf 'n/a\n' >"$rss_file"
    fi
}

printf '| task | native ms | interp ms | python ms | c ms | native KB | interp KB | python KB | c KB |\n'
printf '|------|-----------|-----------|-----------|------|-----------|-----------|-----------|------|\n'

for task in "${TASKS[@]}"; do
    echo "bench: $task" >&2
    workdir="$WORK/$task"
    golden_prefix="$(golden_prefix "$task")"
    mkdir -p "$workdir/data" "$workdir/out"
    if [[ -d "$APH_DIR/data" ]]; then
        cp -a "$APH_DIR/data/." "$workdir/data/"
    fi

    ARGS=()
    if [[ -f "$golden_prefix.args" ]]; then
        mapfile -t ARGS <"$golden_prefix.args"
    fi

    EXPECTED_FILES=()
    if [[ -f "$golden_prefix.files" ]]; then
        mapfile -t EXPECTED_FILES <"$golden_prefix.files"
    fi
    for local_file in "${EXPECTED_FILES[@]}"; do
        mkdir -p "$workdir/$(dirname "$local_file")"
    done

    expected_status=0
    if [[ -f "$golden_prefix.exit" ]]; then
        expected_status="$(<"$golden_prefix.exit")"
    fi

    (
        cd "$workdir"
        nice -n 15 "$ALEPH" build "$golden_prefix.aph" -o "$workdir/prog" \
            >"$workdir/build.out" 2>"$workdir/build.err"
    ) || {
        echo "error: aleph build failed for $task" >&2
        cat "$workdir/build.out" >&2
        cat "$workdir/build.err" >&2
        exit 1
    }

    if ! nice -n 15 "$CC_BIN" -std=c99 -O2 -o "$workdir/prog-c" "$REF_DIR/c/$task.c" -lm \
        >"$workdir/cc.out" 2>"$workdir/cc.err"; then
        echo "error: C reference failed to build for $task" >&2
        cat "$workdir/cc.err" >&2
        exit 1
    fi

    native_cmd=(nice -n 15 "$workdir/prog")
    interp_cmd=(nice -n 15 "$ALEPH" run "$golden_prefix.aph")
    c_cmd=(nice -n 15 "$workdir/prog-c")
    py_cmd=()
    if [[ -n "$PYTHON" ]]; then
        py_cmd=(nice -n 15 "$PYTHON" "$REF_DIR/py/$task.py")
    fi

    verify_engine aleph-native "${native_cmd[@]}"
    verify_engine aleph-interp "${interp_cmd[@]}"
    if (( ${#py_cmd[@]} > 0 )); then
        verify_engine python "${py_cmd[@]}"
    fi
    verify_engine c "${c_cmd[@]}"

    time_engine "$workdir/native.times" "$expected_status" "${native_cmd[@]}"
    time_engine "$workdir/interp.times" "$expected_status" "${interp_cmd[@]}"
    if (( ${#py_cmd[@]} > 0 )); then
        time_engine "$workdir/python.times" "$expected_status" "${py_cmd[@]}"
    fi
    time_engine "$workdir/c.times" "$expected_status" "${c_cmd[@]}"

    rss_engine "$workdir/native.rss" "$expected_status" "${native_cmd[@]}"
    rss_engine "$workdir/interp.rss" "$expected_status" "${interp_cmd[@]}"
    if (( ${#py_cmd[@]} > 0 )); then
        rss_engine "$workdir/python.rss" "$expected_status" "${py_cmd[@]}"
    else
        printf 'n/a\n' >"$workdir/python.rss"
    fi
    rss_engine "$workdir/c.rss" "$expected_status" "${c_cmd[@]}"

    native_ms="$(median_ms "$workdir/native.times")"
    interp_ms="$(median_ms "$workdir/interp.times")"
    python_ms="$(median_ms "$workdir/python.times")"
    c_ms="$(median_ms "$workdir/c.times")"

    native_kb="$(<"$workdir/native.rss")"
    interp_kb="$(<"$workdir/interp.rss")"
    python_kb="$(<"$workdir/python.rss")"
    c_kb="$(<"$workdir/c.rss")"

    printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
        "$task" "$native_ms" "$interp_ms" "$python_ms" "$c_ms" \
        "$native_kb" "$interp_kb" "$python_kb" "$c_kb"
done
