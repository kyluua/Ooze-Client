#!/usr/bin/env bash
# Ooze Client launch-test matrix (TESTING.md §3). Runs real Minecraft 1.21.11 clients and collects
# screenshots + log excerpts as evidence.
#
#   tools/launch-tests.sh [OUTPUT_DIR]        default OUTPUT_DIR=build/launch-evidence
#
# Environment:
#   OOZE_NATIVE_RESOURCES=<abs dir>   Sol's prebuilt resources dir (contains META-INF/ooze/native.properties
#                                     and natives/windows-x86_64/ooze_jni.dll). Passed to Gradle as
#                                     -PoozeNativeResources. On Windows x64 this also enables the
#                                     real-artifact failure cases (DLL removed / corrupted / ABI mismatch).
#
# Linux: runs under xvfb-run (needs xvfb + Mesa). Windows (Git Bash): runs directly.
# Expected engine state defaults inside the test: LOADED on Windows x64, UNSUPPORTED_PLATFORM elsewhere.
# Every case must end with "C0 launch test PASSED"; the script exits non-zero otherwise.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/build/launch-evidence}"
PROJECT=":versions:mc1_21_11"
RUN_DIR="$ROOT/versions/mc1_21_11/build/run"
RES="${OOZE_NATIVE_RESOURCES:-}"
mkdir -p "$OUT"

case "$(uname -s)" in
  Linux*) WRAP=(xvfb-run -a -s "-screen 0 1920x1080x24"); WINDOWS=0 ;;
  MINGW*|MSYS*|CYGWIN*) WRAP=(); WINDOWS=1 ;;
  *) WRAP=(); WINDOWS=0 ;;
esac
ARCH="$(uname -m)"

res_arg() { [[ -n "$1" ]] && echo "-PoozeNativeResources=$1" || true; }

# name | gradle task | extra gradle args | screenshot dir (relative to RUN_DIR)
CASES=(
  "dev-default|runClientGameTest|$(res_arg "$RES")|clientGameTest"
  "dev-missing_library|runClientGameTest|-PoozeEngineFault=missing_library $(res_arg "$RES")|clientGameTest"
  "dev-incompatible|runClientGameTest|-PoozeEngineFault=incompatible $(res_arg "$RES")|clientGameTest"
  "dev-load_failed|runClientGameTest|-PoozeEngineFault=load_failed $(res_arg "$RES")|clientGameTest"
  "prod-default|runProductionClientGameTest|$(res_arg "$RES")|productionClientGameTest"
  "prod-fault-ignored|runProductionClientGameTest|-PoozeEngineFault=incompatible $(res_arg "$RES")|productionClientGameTest"
)

# Real-artifact failure variants: only meaningful where the bridge passes its platform check.
if [[ $WINDOWS -eq 1 && "$ARCH" =~ ^(x86_64|amd64)$ && -n "$RES" ]]; then
  VAR="$OUT/variants"; mkdir -p "$VAR"
  for v in missing corrupt abi; do
    cp -R "$RES" "$VAR/$v"
  done
  rm "$VAR/missing/natives/windows-x86_64/ooze_jni.dll"
  printf 'X' | dd of="$VAR/corrupt/natives/windows-x86_64/ooze_jni.dll" bs=1 seek=4096 conv=notrunc status=none
  sed -i 's/^bridge.abi=.*/bridge.abi=2/' "$VAR/abi/META-INF/ooze/native.properties"
  CASES+=(
    "win-real-missing_library|runClientGameTest|-PoozeNativeResources=$VAR/missing -PoozeExpectEngineState=MISSING_LIBRARY|clientGameTest"
    "win-real-corrupt|runClientGameTest|-PoozeNativeResources=$VAR/corrupt -PoozeExpectEngineState=INCOMPATIBLE|clientGameTest"
    "win-real-abi-mismatch|runClientGameTest|-PoozeNativeResources=$VAR/abi -PoozeExpectEngineState=INCOMPATIBLE|clientGameTest"
  )
fi

failures=0
summary="$OUT/SUMMARY.txt"
{ echo "host: $(uname -s) $ARCH"; echo "native resources: ${RES:-<none>}"; } > "$summary"
for entry in "${CASES[@]}"; do
  IFS='|' read -r name task extra shotdir <<< "$entry"
  log="$OUT/$name.log"
  marker="$OUT/.start-$name"; : > "$marker"
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
  # Only screenshots written by this case (production run dirs are not cleared between runs).
  if [[ -d "$RUN_DIR/$shotdir/screenshots" ]]; then
    while IFS= read -r png; do
      cp "$png" "$OUT/$name--$(basename "$png")"
    done < <(find "$RUN_DIR/$shotdir/screenshots" -name '*.png' -newer "$marker")
  fi
  state="$(grep -m1 -oE 'state=[A-Z_]+' "$log" | head -1 | cut -d= -f2 || true)"
  printf '%-28s %-16s state=%s\n' "$name" "$result" "${state:-?}" | tee -a "$summary"
  grep -E "is OFF|state    :|native engine loaded|Ignoring -D|bootstrap:|bridge probe|bridge fixture|C0 launch test PASSED|AssertionError" "$log" \
    | sed -E 's/^\[[^]]*\] //' > "$OUT/$name.excerpt.txt" || true
done

echo "Evidence in $OUT"
exit $failures
