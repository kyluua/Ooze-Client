#!/usr/bin/env bash
# Ooze Client launch-test matrix (TESTING.md §3) for every advertised Fabric target
# (docs/TARGET_PLATFORM.md §1). Runs real Minecraft clients and collects screenshots and log
# excerpts as evidence.
#
#   tools/launch-tests.sh [OUTPUT_DIR]        default OUTPUT_DIR=build/launch-evidence
#
# Environment:
#   OOZE_NATIVE_RESOURCES=<abs dir>  Sol's prebuilt resources dir (META-INF/ooze/native.properties and
#                                    natives/windows-x86_64/ooze_jni.dll), passed as -PoozeNativeResources.
#                                    On Windows x64 this gives engine=LOADED and enables the real-artifact
#                                    failure cases (DLL removed / corrupted / ABI mismatch).
#   OOZE_TARGETS="1.21.11 26.3"      Optional: only these Minecraft versions (default: all).
#   OOZE_DEV_CASES=0                 Optional: skip the per-adapter development fault cases.
#   OOZE_VALIDATED_STATE=LOADED      Engine state a 26.x production run must reach for its JAR to be
#                                    listed in jars/validated-ooze-clients.json (default LOADED).
#
# Steps:
#   1. ./gradlew buildAll builds every distribution JAR once; they are copied to OUTPUT_DIR/jars and
#      their SHA-256 recorded. These exact files are the ones launched.
#   2. prod-<version>: each shipped JAR runs in a production Fabric environment on every Minecraft
#      version it lists, with its pinned Fabric API. Expected engine: LOADED on Windows x64 with
#      native resources, MISSING_LIBRARY on Windows x64 without them, UNSUPPORTED_PLATFORM elsewhere.
#   3. Per adapter (its compile version): development fault cases, the production run with a fault
#      (release builds must ignore it) and, on Windows x64 with resources, the real-artifact failures.
#
# A case passes only if Gradle succeeds, the log has "C0 launch test PASSED" (title screen reached,
# vanilla Options round trip OK), "Loading Minecraft <version>" and the exact readiness line
# "OOZE_READY v1 screen=title engine=<expected>". The script exits non-zero if any case fails.
#
# Output for Sol's catalog packaging (docs/handoffs/SOL-MULTIVERSION.md): OUTPUT_DIR/jars/ holds the
# tested JARs and validated-ooze-clients.json, listing each 26.x version whose production run passed
# with engine=$OOZE_VALIDATED_STATE (LOADED by default, so only Windows x64 with native resources
# produces rows). 1.21.11 comes from the base package and is not listed.
# Linux runs under xvfb-run (needs xvfb + Mesa). Windows (Git Bash) runs directly.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/build/launch-evidence}"
RES="${OOZE_NATIVE_RESOURCES:-}"
DEV_CASES="${OOZE_DEV_CASES:-1}"
mkdir -p "$OUT/jars"

case "$(uname -s)" in
  Linux*) WRAP=(xvfb-run -a -s "-screen 0 1920x1080x24"); WINDOWS=0 ;;
  MINGW*|MSYS*|CYGWIN*) WRAP=(); WINDOWS=1 ;;
  *) WRAP=(); WINDOWS=0 ;;
esac
ARCH="$(uname -m)"
WINX64=0
if [[ $WINDOWS -eq 1 && "$ARCH" =~ ^(x86_64|amd64)$ ]]; then WINX64=1; fi

if [[ $WINX64 -eq 1 && -n "$RES" ]]; then DEFAULT_STATE=LOADED
elif [[ $WINX64 -eq 1 ]]; then DEFAULT_STATE=MISSING_LIBRARY
else DEFAULT_STATE=UNSUPPORTED_PLATFORM
fi

res_arg() { [[ -n "$1" ]] && echo "-PoozeNativeResources=$1" || true; }
sha() { sha256sum "$1" | cut -d' ' -f1; }

MOD_VERSION="$(grep -E '^mod_version=' "$ROOT/gradle.properties" | cut -d= -f2)"

# minecraft version | Gradle project | JAR family suffix | extra Gradle args for that version
TARGETS=(
  "1.21.11|:versions:mc1_21_11|mc1.21.11|"
  "26.1|:versions:mc26_1|mc26.1|-PoozeMinecraft=26.1"
  "26.1.1|:versions:mc26_1|mc26.1|-PoozeMinecraft=26.1.1"
  "26.1.2|:versions:mc26_1|mc26.1|-PoozeMinecraft=26.1.2"
  "26.2|:versions:mc26_2|mc26.2|-PoozeMinecraft=26.2"
  "26.3|:versions:mc26_3|mc26.3|-PoozeMinecraft=26.3"
)
# Gradle project | its compile version (for the development cases)
ADAPTERS=(
  ":versions:mc1_21_11|1.21.11"
  ":versions:mc26_1|26.1.2"
  ":versions:mc26_2|26.2"
  ":versions:mc26_3|26.3"
)

wanted() { [[ -z "${OOZE_TARGETS:-}" ]] || [[ " $OOZE_TARGETS " == *" $1 "* ]]; }

summary="$OUT/SUMMARY.txt"
{
  echo "host: $(uname -s) $ARCH"
  echo "native resources: ${RES:-<none>}"
  [[ -n "$RES" && -f "$RES/natives/windows-x86_64/ooze_jni.dll" ]] && echo "native library sha256: $(sha "$RES/natives/windows-x86_64/ooze_jni.dll")"
  echo "expected engine state (default): $DEFAULT_STATE"
  echo "source: $(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo '?')$(git -C "$ROOT" diff --quiet 2>/dev/null || echo ' (with local changes)')"
} > "$summary"

echo "== buildAll"
# shellcheck disable=SC2046
"$ROOT/gradlew" --no-daemon -p "$ROOT" buildAll $(res_arg "$RES") > "$OUT/buildAll.log" 2>&1
for f in "$ROOT"/build/dist/ooze-client-"$MOD_VERSION"+mc*.jar; do
  cp "$f" "$OUT/jars/"
  printf 'jar %-42s sha256 %s\n' "$(basename "$f")" "$(sha "$f")" | tee -a "$summary"
done

failures=0
VALIDATED_STATE="${OOZE_VALIDATED_STATE:-LOADED}"
validated_rows=()
# name | Gradle project | task | extra args | expected state | minecraft version
run_case() {
  local name="$1" project="$2" task="$3" extra="$4" expected="$5" minecraft="$6"
  local log="$OUT/$name.log" marker="$OUT/.start-$name"
  local projdir; projdir="$ROOT/$(echo "${project#:}" | tr : /)"
  : > "$marker"
  echo "== $name ($project:$task $extra)"
  set +e
  # shellcheck disable=SC2086
  "${WRAP[@]}" "$ROOT/gradlew" --no-daemon -p "$ROOT" "$project:$task" $extra > "$log" 2>&1
  local code=$?
  set -e
  local result="PASS"
  if [[ $code -ne 0 ]]; then result="FAIL (exit $code)"
  elif ! grep -q "C0 launch test PASSED" "$log"; then result="FAIL (no PASSED line)"
  elif ! grep -q "Loading Minecraft $minecraft with Fabric Loader" "$log"; then result="FAIL (not Minecraft $minecraft)"
  elif ! grep -qE "OOZE_READY v1 screen=title engine=$expected\$" "$log"; then result="FAIL (no OOZE_READY engine=$expected)"
  fi
  [[ $result == PASS ]] || failures=$((failures + 1))
  LAST_RESULT="$result"
  # Only screenshots written by this case (run directories are not cleared between runs).
  if [[ -d "$projdir/build/run" ]]; then
    while IFS= read -r png; do
      cp "$png" "$OUT/$name--$(basename "$png")"
    done < <(find "$projdir/build/run" -path '*/screenshots/*.png' -newer "$marker")
  fi
  printf '%-34s %-34s minecraft=%-8s expected=%s\n' "$name" "$result" "$minecraft" "$expected" | tee -a "$summary"
  grep -E "Loading Minecraft|Production run|OOZE_READY|input/window|graphics backend|SDL video driver|fixture pixel|is OFF|state    :|native engine loaded|Ignoring -D|bootstrap:|bridge probe|bridge fixture|C0 launch test PASSED|AssertionError" "$log" \
    | sed -E 's/^\[[^]]*\] //' > "$OUT/$name.excerpt.txt" || true
}

for entry in "${TARGETS[@]}"; do
  IFS='|' read -r minecraft project family extra <<< "$entry"
  wanted "$minecraft" || continue
  jar="$OUT/jars/ooze-client-$MOD_VERSION+$family.jar"
  run_case "prod-$minecraft" "$project" runProductionClientGameTest \
    "-PoozeProductionJar=$jar $extra $(res_arg "$RES") -PoozeExpectEngineState=$DEFAULT_STATE" "$DEFAULT_STATE" "$minecraft"
  if [[ $LAST_RESULT == PASS && $DEFAULT_STATE == "$VALIDATED_STATE" && $minecraft != 1.21.11 ]]; then
    props="$ROOT/$(echo "${project#:}" | tr : /)/gradle.properties"
    api="$(grep -E '^fabric_api_version=' "$props" | cut -d= -f2)"
    loader="$(grep -E '^loader_version=' "$props" | cut -d= -f2)"
    java="$(grep -E '^java_version=' "$props" | cut -d= -f2)"
    validated_rows+=("$(printf '{"minecraft":"%s","java_major":%s,"loader":"%s","api_version":"%s","file":"%s","sha256":"%s","evidence":"../prod-%s.excerpt.txt"}' \
      "$minecraft" "$java" "$loader" "$api" "$(basename "$jar")" "$(sha "$jar")" "$minecraft")")
  fi
done
manifest="$OUT/jars/validated-ooze-clients.json"
{
  printf '{"schema":1,"targets":['
  (IFS=,; printf '%s' "${validated_rows[*]:-}")
  printf ']}\n'
} > "$manifest"
echo "validated 26.x clients (engine=$VALIDATED_STATE): ${#validated_rows[@]} -> $manifest" | tee -a "$summary"

if [[ "$DEV_CASES" != 0 ]]; then
  if [[ $WINX64 -eq 1 && -n "$RES" ]]; then
    VAR="$OUT/variants"; mkdir -p "$VAR"
    for v in missing corrupt abi; do rm -rf "${VAR:?}/$v"; cp -R "$RES" "$VAR/$v"; done
    rm "$VAR/missing/natives/windows-x86_64/ooze_jni.dll"
    printf 'X' | dd of="$VAR/corrupt/natives/windows-x86_64/ooze_jni.dll" bs=1 seek=4096 conv=notrunc status=none
    sed -i 's/^bridge.abi=.*/bridge.abi=2/' "$VAR/abi/META-INF/ooze/native.properties"
  fi
  for entry in "${ADAPTERS[@]}"; do
    IFS='|' read -r project minecraft <<< "$entry"
    wanted "$minecraft" || continue
    tag="${project##*:}"
    for fault in missing_library incompatible load_failed; do
      state="$(echo "$fault" | tr '[:lower:]' '[:upper:]')"
      run_case "dev-$tag-$fault" "$project" runClientGameTest "-PoozeEngineFault=$fault $(res_arg "$RES")" "$state" "$minecraft"
    done
    jar="$OUT/jars/ooze-client-$MOD_VERSION+$(echo "$tag" | tr _ .).jar"   # mc26_1 -> mc26.1
    run_case "prod-$tag-fault-ignored" "$project" runProductionClientGameTest \
      "-PoozeProductionJar=$jar -PoozeEngineFault=incompatible $(res_arg "$RES") -PoozeExpectEngineState=$DEFAULT_STATE" "$DEFAULT_STATE" "$minecraft"
    if [[ $WINX64 -eq 1 && -n "$RES" ]]; then
      run_case "win-$tag-real-missing_library" "$project" runClientGameTest "-PoozeNativeResources=$VAR/missing -PoozeExpectEngineState=MISSING_LIBRARY" MISSING_LIBRARY "$minecraft"
      run_case "win-$tag-real-corrupt" "$project" runClientGameTest "-PoozeNativeResources=$VAR/corrupt -PoozeExpectEngineState=INCOMPATIBLE" INCOMPATIBLE "$minecraft"
      run_case "win-$tag-real-abi-mismatch" "$project" runClientGameTest "-PoozeNativeResources=$VAR/abi -PoozeExpectEngineState=INCOMPATIBLE" INCOMPATIBLE "$minecraft"
    fi
  done
fi

echo "failures: $failures" | tee -a "$summary"
echo "Evidence in $OUT"
exit $failures
