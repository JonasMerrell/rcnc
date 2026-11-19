"""After rCAD posts: pauses, dwells and marks, then verify_gcode on every final file (verify.json).

- op 2 (and the cycles): stop to blow the carrier bores out before Find bores (add_bores_pause.py);
- aluminium: a chip-clearing stop before the finishing that follows the 9k roughing (add_finish_pause.py);
- 3 s in air after every S change (add_dwell.py: the bog guard learns that speed's no-load speed);
- (RCNC REQUIRE_GUARD) in every program (mark_guard.py)."""
import json, subprocess

def run(*a):
    print(subprocess.run(["python3", *a], capture_output=True, text=True, check=True).stdout.strip())

ALL = ("base-op1.nc", "base-op2.nc", "slider-op1.nc", "slider-op2.nc", "slider-engrave.nc", "base-cycle.nc", "slider-cycle.nc",
       "base-first.nc", "base-last.nc", "slider-first.nc", "slider-last.nc")
has = lambda p, t: t in open(p).read()
# each step only where it isn't in yet (programs re-posted one at a time keep the others' marks)
for p in ("base-op2.nc", "slider-op2.nc", "base-cycle.nc", "slider-cycle.nc", "base-last.nc", "slider-last.nc"):
    if not has(p, "BORES PAUSE"):
        run("add_bores_pause.py", p)
FIN1 = ("(Finish the walls in one full-height pass", "Blow the chips off the bar and out of the gap between the bases")
FIN2 = ("(Finish the channel floor", "Blow the chips out of the two channels and off the top")
for p, marks in (("base-op1.nc", [FIN1]), ("base-op2.nc", [FIN2]), ("base-cycle.nc", [FIN1, FIN2]), ("base-first.nc", [FIN1]), ("base-last.nc", [FIN2])):
    for m, what in marks:
        t = open(p).read()
        if "(END CHIP PAUSE)" not in t[max(0, t.index(m) - 40):t.index(m)]:
            run("add_finish_pause.py", p, m, what)
for p in ALL:
    run("add_dwell.py", p)
for p in ALL:
    if not has(p, "REQUIRE_GUARD"):
        run("mark_guard.py", p)

V = [{"tool": "open_document", "args": {"path": "slider-cam.json"}}]
for cam, path, st in ((0, "base-op1.nc", ["Base op1 bottom"]), (1, "base-op2.nc", None), (2, "slider-op1.nc", ["Slider op1 bottom"]), (3, "slider-op2.nc", None),
                      (4, "slider-engrave.nc", None), (0, "base-cycle.nc", None), (2, "slider-cycle.nc", None),
                      (0, "base-first.nc", ["Base op1 bottom"]), (0, "base-last.nc", ["Base op2 top"]),
                      (2, "slider-first.nc", ["Slider op1 bottom"]), (2, "slider-last.nc", ["Slider op2 top", "Slider op2 engrave"])):
    a = {"cam": cam, "path": path}
    if st:
        a["stations"] = st
    V.append({"tool": "verify_gcode", "args": a})
json.dump(V, open("verify.json", "w"), indent=1)
print("verify.json written")
