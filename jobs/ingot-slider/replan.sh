#!/bin/sh
# Re-plan after a change (e.g. the shim): ./replan.sh slider | base | all
# CAM for the affected part only, then post-processing, then verify_gcode on that part's programs in two
# parallel rcad processes. Needs rCAD with the build and simulation caches (PRs #165, #166).
set -e
cd "$(dirname "$0")"
R=${RCAD:-$HOME/src/cad-rust-app-slider/target/release/rcad}
part=${1:-all}
python3 cam.py
python3 - "$part" <<'PY'
import json, sys
part = sys.argv[1]
S = json.load(open("cam.json"))
OUT = {"cam_info", "simulate_cam", "render_cam", "export_gcode", "export_setup_sheet", "export_cam_simulation"}
skip = {"slider": (0, 1), "base": (2, 3, 4), "all": ()}[part]
T = [s for s in S if s["tool"] != "export_cam_simulation" and not (s["tool"] in OUT and s["args"].get("cam") in skip)]
if part == "base":   # the base's first/last programs come from its job
    pass
json.dump(T, open("cam-replan.json", "w"))
V = json.load(open("verify.json"))
keep = [v for v in V[1:] if part == "all" or v["args"]["path"].startswith(part)]
a = [v for v in keep if any(k in v["args"]["path"] for k in ("op1", "first", "cycle"))]
b = [v for v in keep if v not in a]
json.dump([V[0]] + a, open("verify-a.json", "w")); json.dump([V[0]] + b, open("verify-b.json", "w"))
PY
$R run cam-replan.json > cam.out 2> cam.err
python3 post.py
$R run verify-a.json > verify-a.out 2>/dev/null & $R run verify-b.json > verify-b.out 2>/dev/null & wait
cat verify-a.out verify-b.out | python3 -c '
import json, sys
bad = 0
for l in sys.stdin:
    d = json.loads(l); r = d.get("result", {})
    if "error" in d: print("ERROR", d); bad += 1
    if d.get("tool") == "verify_gcode":
        print(r["path"], "ok" if r["ok"] else "NOT OK", len(r["problems"]), "problems"); bad += not r["ok"]
sys.exit(bad)'
