#!/bin/sh
# tsrun.sh script.json out-prefix: rcad run with a timestamp per step (prefix.times)
R=${RCAD:-/home/jonas/dmc2/eclipse-car/bin/rcad}
$R run "$1" 2> "$2.err" | python3 -u -c '
import sys, time, json
t0 = time.time(); last = t0
out = open(sys.argv[1] + ".out", "w"); tl = open(sys.argv[1] + ".times", "w")
for line in sys.stdin:
    out.write(line); out.flush(); now = time.time()
    try:
        j = json.loads(line); a = j.get("tool"), j.get("step")
    except Exception:
        a = line[:60]
    tl.write(f"{now-last:8.1f} {now-t0:8.1f} {a}\n"); tl.flush(); last = now
' "$2"
