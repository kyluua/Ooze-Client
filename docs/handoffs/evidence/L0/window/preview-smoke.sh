#!/usr/bin/env bash
# Launch the developer preview in a real X window (Xvfb) for each scenario and capture the screen.
set -u
BIN="$1"; OUT="$2"; mkdir -p "$OUT"
for sc in first-run ready; do
  "$BIN" "$sc" > "$OUT/$sc.log" 2>&1 &
  pid=$!
  sleep 6
  import -window root "$OUT/preview-$sc.png" 2>>"$OUT/$sc.log"
  if kill -0 $pid 2>/dev/null; then echo "$sc: window alive after 6s"; kill $pid; wait $pid 2>/dev/null; else echo "$sc: process exited early"; fi
done
