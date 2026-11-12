#!/usr/bin/env bash
# Aleph agent loop: runs opencode headless, one iteration at a time.
# Usage:  loop/run.sh [build|orbis]
# Env:    ALEPH_MODEL   provider/model for opencode (check: opencode models deepseek)
#         MAX_ITER      max iterations, 0 = unlimited (default 0)
#         ITER_TIMEOUT  seconds per iteration (default 3600)
#         SLEEP         seconds between iterations (default 30)
#         NOPROGRESS    stop after N consecutive iterations without a new commit (default 4)
# Stop:   touch STOP   (checked before every iteration and by the agent itself)
set -u

MODE="${1:-build}"
MODEL="${ALEPH_MODEL:-deepseek/deepseek-chat}"
MAX_ITER="${MAX_ITER:-0}"
ITER_TIMEOUT="${ITER_TIMEOUT:-3600}"
SLEEP="${SLEEP:-30}"
NOPROGRESS="${NOPROGRESS:-4}"

case "$MODE" in
  build) PROMPT_FILE="loop/prompt-build.md" ;;
  orbis) PROMPT_FILE="loop/prompt-orbis.md" ;;
  *) echo "usage: $0 [build|orbis]"; exit 2 ;;
esac

cd "$(dirname "$0")/.." || exit 1
command -v opencode >/dev/null || { echo "opencode not found"; exit 1; }
git rev-parse --git-dir >/dev/null 2>&1 || { echo "not a git repo: run git init && git add -A && git commit -m init"; exit 1; }
mkdir -p loop/logs

i=0
stale=0
echo "$(date -Iseconds) start mode=$MODE model=$MODEL" >> loop/logs/runs.txt

while true; do
  if [ -f STOP ]; then echo "$(date -Iseconds) STOP file found" | tee -a loop/logs/runs.txt; break; fi
  if [ "$MODE" = build ] && ! grep -Eq '^- \[ \] T-[0-9]+' TASKS.md; then
    echo "$(date -Iseconds) no open tasks left" | tee -a loop/logs/runs.txt; break
  fi

  # A crashed iteration may leave uncommitted changes: park them so the next run starts clean.
  if [ -n "$(git status --porcelain -- . ':!loop/logs')" ]; then
    git stash push -u -m "aborted-iteration-$(date +%Y%m%d-%H%M%S)" -- . ':!loop/logs' >/dev/null 2>&1
    echo "$(date -Iseconds) stashed leftovers of an aborted iteration" >> loop/logs/runs.txt
  fi

  i=$((i + 1))
  ts="$(date +%Y%m%d-%H%M%S)"
  log="loop/logs/${ts}-${MODE}-${i}.log"
  before="$(git rev-parse HEAD)"

  timeout "$ITER_TIMEOUT" opencode run -m "$MODEL" "$(cat "$PROMPT_FILE")" > "$log" 2>&1
  rc=$?

  after="$(git rev-parse HEAD)"
  if [ "$before" = "$after" ]; then stale=$((stale + 1)); else stale=0; fi
  echo "$(date -Iseconds) iter=$i mode=$MODE rc=$rc commit=$([ "$before" = "$after" ] && echo none || git log -1 --format=%s) stale=$stale" >> loop/logs/runs.txt

  if [ "$stale" -ge "$NOPROGRESS" ]; then
    echo "$(date -Iseconds) $NOPROGRESS iterations without commits: stopping to save budget" | tee -a loop/logs/runs.txt; break
  fi
  if [ "$MAX_ITER" -gt 0 ] && [ "$i" -ge "$MAX_ITER" ]; then
    echo "$(date -Iseconds) reached MAX_ITER=$MAX_ITER" | tee -a loop/logs/runs.txt; break
  fi
  sleep "$SLEEP"
done
