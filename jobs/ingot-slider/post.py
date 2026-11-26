"""After rCAD posts: pauses, dwells, Find stock from the vise location and marks, then verify_gcode on every final file (verify.json).

- op 2 (and the cycles): stop to blow the carrier bores out before Find bores (add_bores_pause.py);
- 3 s in air after every S change (add_dwell.py: the bog guard learns that speed's no-load speed);
- (RCNC REQUIRE_GUARD) in every program (mark_guard.py)."""
import json, subprocess

def run(*a):
    print(subprocess.run(["python3", *a], capture_output=True, text=True, check=True).stdout.strip())

ALL = ("base/base-op1.nc", "base/base-op2.nc", "slider/slider-op1.nc", "base/base-cycle.nc", "slider/slider-cycle.nc",
       "base/base-first.nc", "base/base-last.nc", "slider/slider-first.nc", "slider/slider-last.nc")
has = lambda p, t: t in open(p).read()
# each step only where it isn't in yet (programs re-posted one at a time keep the others' marks)
for p in ("base/base-op2.nc", "base/base-cycle.nc", "slider/slider-cycle.nc", "base/base-last.nc", "slider/slider-last.nc"):
    if not has(p, "BORES PAUSE"):
        run("add_bores_pause.py", p)
# no chip-clearing stops before the aluminium finishes: Jonas, 2026-11-21, the chips didn't build up at 9k
for p in ALL:
    run("add_dwell.py", p)
for p in ALL:
    run("add_vise.py", p)
for p in ALL:
    if not has(p, "REQUIRE_GUARD"):
        run("mark_guard.py", p)

# slider op 1 runs with no shims on the cut left jaws (Jonas, 2026-11-26) and deepens their cut: say so in the header
NOTE = "(NOTE: slider op 1, NO SHIMS: the bar sits on the ledges of the cut left-station jaws [grip 3.33]. T24 goes to Z3.63 beside them and cuts the jaw bars up to 1.07 deeper, 3.8-7 mm in from their faces [Jonas OK 2026-11-26; vise model vise-2026-11-26-left-jaws-cut, bars as after this run])\n"
for p in ("slider/slider-op1.nc", "slider/slider-first.nc", "slider/slider-cycle.nc"):
    t = open(p).read()
    if "NO SHIMS" not in t:
        i = t.index("\n", t.index("(RCNC REQUIRE_GUARD")) + 1
        open(p, "w").write(t[:i] + NOTE + t[i:])

V = [{"tool": "open_document", "args": {"path": "slider-cam.json"}}]
for cam, path, st in ((0, "base/base-op1.nc", ["Base op1 bottom"]), (1, "base/base-op2.nc", None), (2, "slider/slider-op1.nc", ["Slider op1 bottom"]), (0, "base/base-cycle.nc", None), (2, "slider/slider-cycle.nc", None),
                      (0, "base/base-first.nc", ["Base op1 bottom"]), (0, "base/base-last.nc", ["Base op2 top"]),
                      (2, "slider/slider-first.nc", ["Slider op1 bottom"]), (2, "slider/slider-last.nc", ["Slider op2 top", "Slider op2 engrave"])):
    a = {"cam": cam, "path": path, "resolution": 1000}
    if st:
        a["stations"] = st
    V.append({"tool": "verify_gcode", "args": a})
json.dump(V, open("verify.json", "w"), indent=1)
print("verify.json written")
