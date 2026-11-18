"""Stick-out gauge CAM, op 1: op 1a (the calibration passes) and op 1 (the gauge's top side). Writes cam.json for
`rcad run` (rCAD master + PR 151: per-operation rpm/feed, trace lines that start at the seam point's end).

The bar (32 x 32 x 120.7, 6061; measured) lies along Y in the right station, fixed jaw on the left (-X), on the vise's
built-in ledges: Z0 = the Vise reference (the bar's bottom). The jaws grip 4.7; beside them the tool stays 0.3
over them (fixture_clearance). Op 1a probes the stock (Find stock, Z kept from the Vise reference) and sets G54;
op 1 follows it in the same clamping and doesn't probe again (G54 stays).
"""
import json

# the spindle as measured with the VFD at F004 = 150 (op1a v4-v7, 2026-11-18): full torque to 9000 rpm, the power
# falling as (9000 / rpm)^0.76 above; 2600 W x 0.43 = 1118 W at 9000; never-exceed 950 W at 9k, about 765 at 12k,
# 450 at 24k; caution 850 / 684 / 403 (rCAD PR 154)
M = {"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000, "tool_changer": False, "spin_up": 4,
     "spindle_power": 2600, "spindle_efficiency": 0.43, "spindle_rated_rpm": 9000, "spindle_above_rated": 0.76,
     "spindle_never_exceed": 0.85, "spindle_caution": 0.76}
JAW_T, JAW_H, JAW_L = 12.7, 4.7, 95.5
FIXED_R, TABLE_X, LEDGE_Z = 25.5, 47.8, 23.195
BAR, BAR_L = 32.0, 120.7   # measured: 32 x 32, not 1"
CAL = json.load(open("cal_passes.json"))
OLD = json.load(open("old_passes.json"))


def jaws(w):
    return [{"name": "fixed jaw (centre block, -X)", "min": [-w / 2 - JAW_T, -JAW_L / 2, 0], "max": [-w / 2, JAW_L / 2, JAW_H]},
            {"name": "moving jaw (+X)", "min": [w / 2, -JAW_L / 2, 0], "max": [w / 2 + JAW_T, JAW_L / 2, JAW_H]}]


def setup(name, studio, follows=None, stock=None):
    a = {"post": "rcnc", "machine": M, "max_rpm": 24000, "clearance": 5, "retract": 2, "fixture_clearance": 0.8, "probe": {"z_reference": "Vise"},
         "name": name, "studio": studio, "orient": "top", "origin": "part_bottom_center", "on_table": [FIXED_R + BAR / 2 - TABLE_X, 0, LEDGE_Z], "fixtures": jaws(BAR)}
    if stock:
        a["stock"] = stock
    if follows:
        a["follows"] = follows
    return {"tool": "create_cam", "args": a}


def tool(c, which):
    T = {
        1: {"preset": "ALUMINUM", "name": "ALUMINUM chamfer 90", "number": 1, "holder": "ER16 collet chuck", "stickout": 22, "rpm": 24000, "feed": 1850, "plunge_feed": 200},
        24: {"preset": "3 FLUTE DLC 1/4", "name": "3 FLUTE DLC 1/4", "number": 24, "holder": "ER16 collet chuck", "stickout": 24, "rpm": 24000, "feed": 2400, "plunge_feed": 330, "stepdown": 3, "stepover": 0.4},
        11: {"preset": "6mm 2 flute ball end mill", "name": "6 mm 2FL ball (carbide)", "number": 11, "material": "carbide", "holder": "ER16 collet chuck", "stickout": 21, "rpm": 24000, "feed": 3000, "plunge_feed": 600, "stepdown": 1, "stepover": 0.033},
        99: {"preset": "Touch probe", "name": "YIDOFENG 3D touch probe", "number": 99, "diameter": 2.0, "flute_length": 12, "holder": "ER16 collet chuck"},
    }[which]
    return {"tool": "cam_tool", "args": {"cam": c, **T}}


def op(c, kind, name, **a):
    a.update({"cam": c, "kind": kind, "name": name})
    return {"tool": "cam_op", "args": a}


def out(c, stem):
    return [{"tool": "cam_info", "args": {"cam": c}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim.png", "resolution": 900}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim-top.png", "view": "top", "resolution": 900}},
            {"tool": "render_cam", "args": {"cam": c, "path": f"{stem}-paths.png"}},
            {"tool": "export_gcode", "args": {"cam": c, "path": f"{stem}.nc"}},
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{stem}.html"}}]


S = [{"tool": "open_document", "args": {"path": "gauge.json"}},
     {"tool": "import_tool_library", "args": {"path": "../tools/fusion-library.csv", "material": "carbide"}}]

# ---- what the first run on this bar cut (not posted): op1a v1's C1-C3, 6 deep from +X; C4 broke the tool there ----
S += [setup("Already cut (first run)", 3, stock={"size": [BAR, BAR_L, BAR], "bottom": 0.0, "material": "6061-T6"})]
S += [tool(0, 1), tool(0, 24), tool(0, 99)]
for k, (sid, xc) in enumerate(OLD):
    S.append(op(0, "trace", f"First run C{k + 1} (already cut)", tool=24, sketches=[sid], side="both", top=BAR, depth=6.0, stepdown=6.0, seam=[0, -1000.0]))

# ---- op 1a: Find stock, then the calibration passes (studio 5: the raw bar with a line per pass) ----
S += [setup("Op 1a calibration", 3, follows="Already cut (first run)")]
S += [op(1, "probe", "Probe stock top", tool=99, probing={"cycle": "z", "depth": 8, "tolerance": 3.0}),
      op(1, "probe", "Probe stock centre", tool=99, probing={"cycle": "boss", "depth": 8, "tolerance": 3.0})]
for sid, pid, what, w, d, f, rpm, xc, depth, seam_y in CAL:
    S.append(op(1, "trace", f"{pid} {what}: {w:g} x {d:g} F{f} S{rpm}", tool=24, sketches=[sid], side="both", top=BAR, depth=depth, stepdown=depth,
                rpm=rpm, feed=f, seam=[0, seam_y]))
# the 12k and 9k ladders that ran after it (op1a v5 passes 1-7, v6 passes 1-4): -X side, 12 deep at Z20
SP_ = json.load(open("straight_passes.json"))
for k, (sid, x) in enumerate(SP_["ladders"]):
    S.append(op(1, "trace", f"Ladder {k + 1} (ran): x {x:+.3f}, 12 deep", tool=24, sketches=[sid], side="both", top=BAR, depth=12.0, stepdown=12.0, seam=[x, 1000.0]))
S += out(1, "op1a-calibrate")

# ---- op 1: the gauge's top side (studio 3: gauge 3 up on the carrier, end lugs with bores), no probing ----
S += [setup("Op 1 top", 3, follows="Op 1a calibration")]
S += [tool(2, 11)]   # the tool table is the document's: T1 and T24 are there from op 1a
# straight passes (passes.py): step over <= 1 mm, bulk <= ~430 W by the model, each climbing (material on the left)
# the bulk at S9000 (the spindle's strongest speed with the VFD at F004 = 150): straight passes, step over <= 1 mm,
# about 620 W by the model (caution 650, never-exceed 760 at 9k); each climbs (material on the left)
SP = json.load(open("straight_passes.json"))
# Jonas (2026-11-18): climb only, laps all the way round, no strips: the bar's top down to Z19.2 with adaptive
# clearing (two 6.4 levels at S9000, 0.15 x D step over: laps that grow from the open ends and follow the outline)
S.append(op(2, "adaptive", "Clear the bar's top Z32 to Z19.2: climbing adaptive laps, two 6.4 levels (S9000 F3000, 650 W target)", tool=24, top=32.0, bottom=19.2, stepdown=6.4, stepover=0.15, leave=0.0, leave_floor=0.0, rpm=9000, feed=3000, chip_thinning=True, power_target=650, power_cap=850))
S += [op(2, "face", "Finish the gauge's top: face z 19.2 to 19, one 0.2 level", tool=24, top=19.2, stepdown=0.2, stepover=0.4)]
S += [
    op(2, "adaptive", "Rough the thumb dish", tool=24, top=19.0, bottom=15.3, stepdown=1.0, stepover=0.1, leave=0.2, leave_floor=0.2, rest=True),
    op(2, "trace", "Rough the top slots and their end reliefs, 0.2 left on the walls (1 mm levels, F1000: the first lap is a full-width cut; floors to depth)", tool=24, sketches=[3], side="inside", top=19.0, depth=6.0, stepdown=1.0, leave=0.2, feed=1000, plunge_feed=250),
    op(2, "adaptive", "Rough round the gauge Z19 to Z8 (lugs and sides): climbing adaptive laps following the outline, two 5.5 levels, 0.84 step over (S9000 F3000, 650 W target)", tool=24, top=19.0, bottom=8.0, stepdown=5.5, stepover=0.132, leave=0.3, leave_floor=0.0, rest=True, rpm=9000, feed=3000, chip_thinning=True, power_target=650, power_cap=850),
    op(2, "adaptive", "Rough the walls Z8 to Z5.5 beside the jaws (0.64 step over, 1.5 levels)", tool=24, top=8.0, bottom=5.5, stepdown=1.5, stepover=0.1, leave=0.3, leave_floor=0.0, rest=True),
    op(2, "trace", "Finish the slot walls and the relief stop walls, one full-depth 6 mm pass round their outline at F1200 (starts at the slot's mouth; the relief corners carry more)", tool=24, sketches=[3], side="inside", top=19.0, depth=6.0, stepdown=6.0, leave=0.0, feed=1200),
    op(2, "contour", "Finish the walls and end faces Z19 to Z8, one full-height pass (seam mid -X side)", tool=24, side="outside", top=19.0, bottom=8.0, stepdown=12, seam=[-20, 0]),
    op(2, "contour", "Finish the walls Z8 to Z5.5 (lugs' outsides too) at F1200 (the lug corners carry more)", tool=24, side="outside", top=8.0, bottom=5.5, stepdown=3.5, seam=[-20, 0], feed=1200),
    op(2, "bore", "Two Ø8 bores through the lugs (op 2 finds them)", tool=24, points=[[0, 53.5], [0, -53.5]], thread_diameter=8.0, bottom=-0.5),
    op(2, "scallop", "Finish the thumb dish (0.2 stepover = 0.033 D)", tool=11, stepover=0.033, slope_min=3, leave=0.0),
    op(2, "trace", "Engrave the top numbers 0.3 deep", tool=1, sketches=[9], depth=0.3, stepdown=0.3),
    op(2, "deburr", "Edge break 0.3 on the two bore mouths in the lugs (op 2 probes these bores)", tool=1, chamfer=0.3, tip_offset=0.7, edges=[[4.0, 53.5, 8.0], [4.0, -53.5, 8.0]]),
    op(2, "chamfer", "Edge break 0.3 on every edge of the gauge's top: outline (op 2 sits it on the 1.9 ledges), slots, reliefs, dish rim", tool=1, chamfer=0.3, side="both", tip_offset=1.0),
]
S += [{"tool": "edit_cam", "args": {"cam": 2, "stay_down": True}}]
S += out(2, "op1-top")

json.dump(S, open("cam_loops3.json", "w"), indent=1)
print(len(S), "steps")
