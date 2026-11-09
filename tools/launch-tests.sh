#!/usr/bin/env bash
# Ooze Client launch-test matrix (TESTING.md §3). Runs real Minecraft 1.21.11 clients and collects
# screenshots + log excerpts as evidence.
#
#   tools/launch-tests.sh [OUTPUT_DIR]        default OUTPUT_DIR=build/launch-evidence
#
# Linux: runs under xvfb-run (needs xvfb + Mesa). Windows (Git Bash): runs directly.
# Every case must end with "C0 launch test PASSED"; the script exits non-zero otherwise.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/build/launch-evidence}"
PROJECT=":versions:mc1_21_11"
RUN_DIR="$ROOT/versions/mc1_21_11/build/run"
mkdir -p "$OUT"

if [[ "$(uname -s)" == Linux* ]]; then
  WRAP=(xvfb-run -a -s "-screen 0 1920x1080x24")
else
  WRAP=()
fi

# name | gradle task | extra gradle args | screenshot dir (relative to RUN_DIR)
CASES=(
  "dev-not_bundled|runClientGameTest||clientGameTest"
  "dev-missing_library|runClientGameTest|-PoozeEngineFault=missing_library|clientGameTest"
  "dev-unsupported_platform|runClientGameTest|-PoozeEngineFault=unsupported_platform|clientGameTest"
  "dev-incompatible|runClientGameTest|-PoozeEngineFault=incompatible|clientGameTest"
  "dev-load_failed|runClientGameTest|-PoozeEngineFault=load_failed|clientGameTest"
  "prod-not_bundled|runProductionClientGameTest||productionClientGameTest"
  "prod-fault-ignored|runProductionClientGameTest|-PoozeEngineFault=incompatible|productionClientGameTest"
)

failures=0
summary="$OUT/SUMMARY.txt"
: > "$summary"
for entry in "${CASES[@]}"; do
  IFS='|' read -r name task extra shotdir <<< "$entry"
  log="$OUT/$name.log"
  echo "== $name ($task $extra)"
  set +e
  # shellcheck disable=SC2086
  "${WRAP[@]}" "$ROOT/gradlew" --no-daemon -p "$ROOT" "$PROJECT:$task" $extra > "$log" 2>&1
  code=$?
  set -e
  if [[ $code -eq 0 ]] && grep -q "C0 launch test PASSED" "$log"; then
    result="PASS"
  else
    result="FAIL (exit $code)"
    failures=$((failures + 1))
  fi
  for png in "$RUN_DIR/$shotdir"/screenshots/*.png; do
    [[ -e "$png" ]] && cp "$png" "$OUT/$name.png"
  done
  state="$(grep -m1 -oE 'state    : [A-Z_]+' "$log" | awk '{print $3}' || true)"
  printf '%-26s %-6s state=%s\n' "$name" "$result" "${state:-?}" | tee -a "$summary"
  grep -E "is OFF|state    :|Ignoring -D|bootstrap:|C0 launch test PASSED|AssertionError" "$log" \
    | sed -E 's/^\[[^]]*\] //' > "$OUT/$name.excerpt.txt" || true
done

echo "Evidence in $OUT"
exit $failures
