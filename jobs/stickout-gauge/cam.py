"""Stick-out gauge CAM, op 1: op 1a (the calibration passes) and op 1 (the gauge's top side). Writes cam.json for
`rcad run` (rCAD master + PR 151: per-operation rpm/feed, trace lines that start at the seam point's end).

The bar (25.4 x 25.4 x 120.7, 6061) lies along Y in the right station, fixed jaw on the left (-X), on the vise's
built-in ledges: Z0 = the Vise reference (the bar's bottom). The jaws grip 4.7; beside them the tool stays 0.3
over them (fixture_clearance). Op 1a probes the stock (Find stock, Z kept from the Vise reference) and sets G54;
op 1 follows it in the same clamping and doesn't probe again (G54 stays).
"""
import json

M = {"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000, "tool_changer": False, "spin_up": 4,
     "spindle_power": 1500, "spindle_efficiency": 0.78, "spindle_rated_rpm": 24000}
JAW_T, JAW_H, JAW_L = 12.7, 4.7, 95.5
FIXED_R, TABLE_X, LEDGE_Z = 25.5, 47.8, 23.195
BAR, BAR_L = 25.4, 120.7
CAL = json.load(open("cal_passes.json"))


def jaws(w):
    return [{"name": "fixed jaw (centre block, -X)", "min": [-w / 2 - JAW_T, -JAW_L / 2, 0], "max": [-w / 2, JAW_L / 2, JAW_H]},
            {"name": "moving jaw (+X)", "min": [w / 2, -JAW_L / 2, 0], "max": [w / 2 + JAW_T, JAW_L / 2, JAW_H]}]


def setup(name, studio, follows=None, stock=None):
    a = {"post": "rcnc", "machine": M, "max_rpm": 24000, "clearance": 5, "retract": 2, "fixture_clearance": 0.3, "probe": {"z_reference": "Vise"},
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

# ---- op 1a: Find stock, then the calibration passes (studio 5: the raw bar with a line per pass) ----
S += [setup("Op 1a calibration", 3, stock={"size": [BAR, BAR_L, BAR], "bottom": 0.0, "material": "6061-T6"})]
S += [tool(0, 1), tool(0, 24), tool(0, 99)]   # T1 first in every setup: cam_op's tool takes ids first
S += [op(0, "probe", "Probe stock top", tool=99, probing={"cycle": "z", "depth": 8, "tolerance": 3.0}),
      op(0, "probe", "Probe stock centre", tool=99, probing={"cycle": "boss", "depth": 8, "tolerance": 3.0})]
for sid, pid, what, w, d, f, rpm, xc, depth, seam_y in CAL:
    S.append(op(0, "trace", f"{pid} {what}: {w:g} x {d:g} F{f} S{rpm}", tool=24, sketches=[sid], side="both", top=BAR, depth=depth, stepdown=depth,
                rpm=rpm, feed=f, seam=[0, seam_y]))
S += out(0, "op1a-calibrate")

# ---- op 1: the gauge's top side (studio 3: gauge 3 up on the carrier, end lugs with bores), no probing ----
S += [setup("Op 1 top", 3, follows="Op 1a calibration")]
S += [tool(1, 1), tool(1, 24), tool(1, 11)]
S += [
    op(1, "face", "Face the top to Z19 (what the calibration left of the band)", tool=24, stepdown=1.0),
    op(1, "contour", "Rough the walls to Z5 and the lugs to Z8 (extra passes at 25% of Ø outside the wall)", tool=24, side="outside", bottom=5.0, stepdown=7.0, leave=0.3, finish_lap=0, seam=[-20, 0]),
    op(1, "clear", "Cut the top slots 6.8 wide to size (1.5 mm levels, slow: a full-width cut)", tool=24, stepdown=1.5, leave=0.0, leave_floor=0.0, rest=True, feed=1000, plunge_feed=250),
    op(1, "adaptive", "Rough the thumb dish", tool=24, bottom=15.0, stepdown=3.5, stepover=0.1, leave=0.2, leave_floor=0.2, rest=True),
    op(1, "contour", "Finish the walls and end faces Z19 to Z8, one full-height pass (seam mid -X side)", tool=24, side="outside", bottom=8.0, stepdown=12, seam=[-20, 0]),
    op(1, "contour", "Finish the walls Z8 to Z5 (lugs' outsides too)", tool=24, side="outside", top=8.0, bottom=5.0, stepdown=3.5, seam=[-20, 0]),
    op(1, "bore", "Two Ø8 bores through the lugs (op 2 finds them)", tool=24, points=[[0, 52], [0, -52]], thread_diameter=8.0, bottom=-0.5),
    op(1, "scallop", "Finish the thumb dish (0.2 stepover = 0.033 D)", tool=11, stepover=0.033, slope_min=3, leave=0.0),
    op(1, "trace", "Engrave the top numbers 0.3 deep", tool=1, sketches=[9], depth=0.3, stepdown=0.3),
    op(1, "chamfer", "Chamfer 0.4 every top edge (outline, slots, dish rim, lugs)", tool=1, chamfer=0.4, side="both", tip_offset=1.0),
]
S += out(1, "op1-top")
S += [{"tool": "save_document", "args": {"path": "gauge-cam.json"}}]
json.dump(S, open("cam.json", "w"), indent=1)
print(len(S), "steps")
