#!/usr/bin/env bash
set -u
BIN="$1"; OUT="$2"; mkdir -p "$OUT"
LOCALAPPDATA="$OUT/localappdata" "$BIN" > "$OUT/real.log" 2>&1 &
pid=$!
sleep 5
import -window root "$OUT/real-binary-linux.png"
kill -0 $pid 2>/dev/null && echo "window alive" || echo "exited"
kill $pid 2>/dev/null; wait $pid 2>/dev/null
ls -la "$OUT/localappdata" 2>&1 | head -3
