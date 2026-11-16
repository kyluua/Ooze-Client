#!/usr/bin/env bash
set -u
BIN="$1"; OUT="$2"; mkdir -p "$OUT"
"$BIN" ready > "$OUT/keys.log" 2>&1 &
pid=$!
sleep 5
WID=$(xdotool search --name "Ooze Launcher" | head -1)
xdotool windowfocus --sync "$WID" 2>/dev/null; xdotool mousemove 1100 760
for n in 1 2 3 4 5; do xdotool key --window "$WID" Tab; sleep 0.4; import -window root "$OUT/tab-$n.png"; done
xdotool key --window "$WID" Return; sleep 1.2; import -window root "$OUT/after-enter.png"
kill $pid; wait $pid 2>/dev/null
