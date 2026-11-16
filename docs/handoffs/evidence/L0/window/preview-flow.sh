#!/usr/bin/env bash
# Drive the developer preview through sign-in -> install -> play with real X input events.
set -u
BIN="$1"; OUT="$2"; mkdir -p "$OUT"
"$BIN" signed-out > "$OUT/flow.log" 2>&1 &
pid=$!
sleep 5
WID=$(xdotool search --name "Ooze Launcher" | head -1); echo "window id: $WID"
xdotool mousemove 292 422 click 1; sleep 1.5; import -window root "$OUT/flow-1-browser.png"
sleep 5; import -window root "$OUT/flow-2-signed-in.png"
xdotool mousemove 292 422 click 1; sleep 4; import -window root "$OUT/flow-3-installing.png"
sleep 9; import -window root "$OUT/flow-4-installed.png"
# Keyboard: Tab to Play and press Enter twice quickly (double activation).
xdotool mousemove 1000 700
for i in 1 2 3 4 5 6; do xdotool key Tab; sleep 0.2; done
import -window root "$OUT/flow-5-focus.png"
xdotool mousemove 292 422 click --repeat 2 --delay 60 1; sleep 7; import -window root "$OUT/flow-6-running.png"
kill -0 $pid 2>/dev/null && echo "alive" ; kill $pid; wait $pid 2>/dev/null
