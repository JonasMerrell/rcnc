"""Ingot slider CAM: op 1 and op 2 for the 6061 bases and the C360 sliders, two parts per blank. Writes cam.json
for `rcad run` (the rCAD with fixture_clearance, jaw_x/jaw_at/find_size and deburr tip_offset).

Z0 is the vise's built-in ledges (rcnc's Vise reference) except brass op 1, which lies on a 1 mm shim and takes Z
from the probed top. The jaws grip 4.7 above the ledges; next to them the tool stays 0.3 over their tops
(fixture_clearance), because the stock beside the part is narrower than the tool.
"""
import json

M = {"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000, "tool_changer": False, "spin_up": 4,
     "spindle_power": 1500, "spindle_efficiency": 0.78, "spindle_rated_rpm": 24000}
JAW_T, JAW_H, JAW_L = 12.7, 4.7, 95.5
# Two-station cycle: op 1 (left station) runs in G54, op 2 and the engraving (right station) in G55. The op 1
# setup leads an rCAD job (`with`): export_gcode on it posts both stations as one program (cycle()), and
# `stations` posts op 1 alone. The machine models have both stations' jaws closed (vise_machines.py).
# stations: the vise's centre block is the fixed jaw of both. Left station: fixed jaw on its +X side (op 1),
# right station: fixed jaw on its -X side (op 2, the station the earlier jobs used). Table middle x 47.8.
FIXED_L, FIXED_R, TABLE_X, LEDGE_Z = -7.8, 25.5, 47.8, 23.195
# on_table z puts the bottom of the stock box on the ledges; an op 2 stock box (the op 1 blank) reaches the
# op 1 facing below the part's bottom (0.5 alu, 0.2 brass), so those setups sit that much lower

# adaptive links run at the machine max feed, and rCAD links over "cleared" ground that sometimes still cuts
# (guidelines: open adaptive bugs): cap it in brass so such a link is no faster than 1.2 x the cutting feed
BRASS_LINK = 1600
BRASS = {"rpm": 18000, "feed": 1350, "finish": 1000, "plunge": 250, "ch_feed": 1000}
ALU = {"rpm": 22500, "feed": 2350, "finish": 1900, "plunge": 330, "ch_feed": 1850}


def jaws(w, fixed_side, z0=0.0, extra=()):
    """Jaw boxes (work coordinates) for a part or blank w wide centred on X0, gripping from z0 to z0 + 4.7."""
    fx = "fixed jaw (centre block)"
    mv = "moving jaw"
    left, right = (fx, mv) if fixed_side < 0 else (mv, fx)
    return [{"name": f"{left} (-X)", "min": [-w / 2 - JAW_T, -JAW_L / 2, z0], "max": [-w / 2, JAW_L / 2, z0 + JAW_H]},
            {"name": f"{right} (+X)", "min": [w / 2, -JAW_L / 2, z0], "max": [w / 2 + JAW_T, JAW_L / 2, z0 + JAW_H]}] + list(extra)


def tools(c, f, chamfer=True):
    """The chamfer mill first: cam_op's tool takes an id or a number and ids win, so T1 must be id 1."""
    t = []
    if chamfer:
        t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "ALUMINUM", "name": "ALUMINUM chamfer 90", "number": 1, "holder": "ER16 collet chuck", "stickout": 22,
                                               "rpm": f["rpm"], "feed": f["ch_feed"], "plunge_feed": 200}})
    t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "3 FLUTE DLC 1/4", "name": "3 FLUTE DLC 1/4", "number": 24, "holder": "ER16 collet chuck", "stickout": 24,
                                           "rpm": f["rpm"], "feed": f["feed"], "plunge_feed": f["plunge"], "stepdown": 3, "stepover": 0.4}})
    t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "Touch probe", "name": "YIDOFENG 3D touch probe", "number": 99, "diameter": 2.0, "flute_length": 12, "holder": "ER16 collet chuck"}})
    return t


def probe(c, depth, tol, **extra):
    p = {"cycle": "boss", "depth": depth, "tolerance": tol}
    p.update(extra)
    return [{"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": "Probe stock top", "probing": {"cycle": "z", "depth": depth, "tolerance": tol}}},
            {"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": "Probe stock centre" + (" (X from the fixed jaw)" if "jaw_x" in extra else ""), "probing": p}}]


def find_bores(c, depth):
    """The two carrier bores op 1 cut: rcnc Find bores sets X/Y from them before any roughing."""
    return [{"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": f"Probe carrier bore {'AB'[i]}", "at": xy,
             "probing": {"cycle": "bore", "on_part": True, "size": 7.5, "depth": depth, "tolerance": 0.1, "max_turn": 0.5}}} for i, xy in enumerate(CARRIER)]


def op(c, kind, name, **a):
    a.update({"cam": c, "kind": kind, "name": name})
    return {"tool": "cam_op", "args": a}


def out(c, stem, only=None):
    """Simulate, render and post setup c; `only`: for a job's lead, just these of its setups (op 1 alone)."""
    k = {"stations": only} if only else {}
    return [{"tool": "cam_info", "args": {"cam": c}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim.png", "resolution": 900, **k}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim-top.png", "view": "top", "resolution": 900, **k}},
            {"tool": "render_cam", "args": {"cam": c, "path": f"{stem}-paths.png"}},
            {"tool": "export_gcode", "args": {"cam": c, "path": f"{stem}.nc", **k}},
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{stem}.html", **k}}]


def cycle(c, stem):
    """The whole job c leads: both stations in one program, simulation, sheet and playback."""
    return [{"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim.png", "resolution": 1200}},
            {"tool": "export_gcode", "args": {"cam": c, "path": f"{stem}.nc"}},
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{stem}.html"}},
            {"tool": "export_cam_simulation", "args": {"cam": c, "path": f"{stem}-sim.html", "resolution": 700}}]


def setup(name, studio, mat, stock=None, follows=None, orient="top", origin="part_bottom_center", station_x=0.0, fixtures=(), zref="Vise", z=LEDGE_Z, model="DMC2", max_feed=3000, wcs=1, with_=()):
    """model: the DMC2 machine with the vise's moving jaw closed on this work (vise_machines.py)."""
    a = {"post": "rcnc", "machine": dict(M, model=model, max_feed=max_feed), "max_rpm": 24000, "clearance": 5, "retract": 2, "fixture_clearance": 0.3, "work_offset": wcs, "probe": {"z_reference": zref},
         "name": name, "studio": studio, "orient": orient, "origin": origin, "on_table": [station_x - TABLE_X, 0, z], "fixtures": list(fixtures)}
    if stock:
        a["stock"] = stock
    if follows:
        a["follows"] = follows
    if with_:
        a["with"] = list(with_)
    return {"tool": "create_cam", "args": a}


S = [{"tool": "open_document", "args": {"path": "slider.json"}},
     {"tool": "import_tool_library", "args": {"path": "../tools/fusion-library.csv", "material": "carbide"}}]

# ======================= 6061 bases: blank 1/2 x 1 x ~100, op 1 bottom up, left station =======================
W, BW = 25.4, 25.0
# two bores through the carrier in the 8 mm gap between the parts: op 2 probes them (Find bores) before roughing.
# They are symmetric about X0, so the flip can go either way.
CARRIER = [[-5.0, 0.0], [5.0, 0.0]]
S += [setup("Base op1 bottom", 3, "6061-T6", stock={"size": [W, 100, 12.7], "bottom": 3.2, "material": "6061-T6"}, orient="bottom", origin="stock_bottom_center",
            station_x=FIXED_L - W / 2, model="DMC2-twostation-base", with_=["Base op2 top"],
            fixtures=jaws(W, +1))]
S += tools(0, ALU) + probe(0, 3, 1.5) + [
    op(0, "face", "Face the bar top (the bases' bottoms) 0.5", tool=24, stepdown=0.5),
    op(0, "adaptive", "Rough round both bases and the gap to Z6.5 (0.6 mm stepover)", tool=24, bottom=6.5, stepdown=2.85, stepover=0.095, leave=0.3, leave_floor=0.2),
    op(0, "contour", "Finish the walls, one full-height pass (op 2 grips these)", tool=24, side="outside", bottom=6.5, stepdown=10, lead=2, ramp_angle=90),
    op(0, "bore", "Two locating bores 7.5 through the carrier between the bases (op 2 finds them)", tool=24, points=CARRIER, thread_diameter=7.5, bottom=-0.3),
    op(0, "trace", "Rough the sawn ends off to Z2.5", tool=24, sketches=[13], side="inside", leave=0.2, depth=10.2, stepdown=5.1),
    op(0, "trace", "Finish the ends 98 apart, full height (op 2 finds Y on them)", tool=24, sketches=[13], side="inside", leave=0.0, depth=10.2, stepdown=10.2),
    op(0, "chamfer", "Edge break 0.3 the bottom outline (op 2 sits it on the 1.9 ledges)", tool=1, chamfer=0.3, side="outside", tip_offset=1.0),
 ] + out(0, "base-op1", ["Base op1 bottom"])

# op 2: flipped (end over end), finished bottom on the ledges, the op 1 walls in the jaws, right station
S += [setup("Base op2 top", 3, "6061-T6", follows="Base op1 bottom", z=LEDGE_Z - 0.5, station_x=FIXED_R + BW / 2, fixtures=jaws(BW, -1), model="DMC2-twostation-base", wcs=2)]
S += tools(1, ALU) + probe(1, 4, 1.0, find_size=[W, 98]) + find_bores(1, 2.0) + [
    op(1, "face", "Face to 9 (3.2)", tool=24, stepdown=1.6),
    op(1, "adaptive", "Rough the channel and outside to Z6 (parts still joined below)", tool=24, bottom=6.0, stepdown=3.0, stepover=0.095, leave=0.3, leave_floor=0.2),
    op(1, "clear", "Finish the channel floor (Z6)", tool=24, top=6.1, bottom=6.0, stepdown=0.1, stepover=0.4, leave=0.0, leave_floor=0.0, rest=False),
    op(1, "contour", "Finish the rails, both sides, Z9 to Z6", tool=24, side="outside", top=9.0, bottom=6.0, stepdown=10, lead=2, ramp_angle=90),
    op(1, "bore", "Magnet pockets 8.05 x 3.15 (helical, finish floor)", tool=24),
    op(1, "adaptive", "Rest rough below Z6: separates the two bases (light)", tool=24, bottom=5.02, stepdown=1.0, stepover=0.095, leave=0.3, leave_floor=0.2, rest=True),
    op(1, "contour", "Finish the outline 9 to 5, one full-height pass", tool=24, side="outside", bottom=5.02, stepdown=10, lead=2, ramp_angle=90),
    op(1, "deburr", "Chamfer 1.5 the rail tops outside", tool=1, chamfer=1.5, tip_offset=1.0,
       edges=[[s * BW / 2, cy, 9.0] for s in (1, -1) for cy in (-26.5, 26.5)]),
    op(1, "deburr", "Edge break 0.3: channel edges, floor ends, pocket rims", tool=1, chamfer=0.3, tip_offset=0.7,
       edges=[[s * 10.0, cy, 9.0] for s in (1, -1) for cy in (-26.5, 26.5)] + [[0, cy + s * 22.5, 6.0] for s in (1, -1) for cy in (-26.5, 26.5)]
       + [[4.025, cy + m, 6.0] for m in (-15, 0, 15) for cy in (-26.5, 26.5)]),
] + out(1, "base-op2") + cycle(0, "base-cycle")

# ======================= C360 sliders: blank 3/8 x 1 x ~100 on a 1 mm shim, op 1 bottom up ======================
SW = 19.6
shims = [{"name": f"1 mm shim strip (25 x 39) under the {e} slider, op 1 only", "min": [-12.5, lo, -1.0], "max": [12.5, lo + 39, 0]} for e, lo in (("-Y", -45), ("+Y", 6))]
S += [setup("Slider op1 bottom", 4, "Brass C360", stock={"size": [W, 100, 9.5], "bottom": 1.3, "material": "Brass C360"}, orient="bottom", origin="stock_bottom_center",
            station_x=FIXED_L - W / 2, fixtures=jaws(W, +1, z0=-1.0, extra=shims), zref="", model="DMC2-twostation-slider", with_=["Slider op2 top", "Slider op2 engrave"], max_feed=BRASS_LINK)]
S += tools(2, BRASS) + probe(2, 3, 1.5) + [
    op(2, "face", "Face the bar top (the sliders' bottoms) 0.2", tool=24, stepdown=0.2),
    op(2, "adaptive", "Rough round both sliders and the gap to Z4 (0.5 mm stepover)", tool=24, bottom=4.0, stepdown=5.3, stepover=0.08, leave=0.3, leave_floor=0.2),
    op(2, "contour", "Finish the walls, one full-height pass (op 2 grips these)", tool=24, side="outside", bottom=4.0, stepdown=10, lead=2, ramp_angle=90),
    op(2, "bore", "Magnet pockets 8.05 x 3.15 (helical, finish floor)", tool=24),
    op(2, "bore", "Two locating bores 7.5 through the carrier between the sliders (op 2 finds them)", tool=24, points=CARRIER, thread_diameter=7.5, bottom=-0.3),
    op(2, "trace", "Rough the sawn ends off to Z1", tool=24, sketches=[11], side="inside", leave=0.2, depth=8.5, stepdown=4.25),
    op(2, "trace", "Finish the ends 98 apart, full height (op 2 finds Y on them)", tool=24, sketches=[11], side="inside", leave=0.0, depth=8.5, stepdown=8.5),
    op(2, "chamfer", "Edge break 0.3: bottom outline and pocket rims", tool=1, chamfer=0.3, side="both", tip_offset=1.0),
 ] + out(2, "slider-op1", ["Slider op1 bottom"])

S += [setup("Slider op2 top", 4, "Brass C360", follows="Slider op1 bottom", z=LEDGE_Z - 0.2, station_x=FIXED_R + SW / 2, fixtures=jaws(SW, -1), model="DMC2-twostation-slider", max_feed=BRASS_LINK, wcs=2)]
S += tools(3, BRASS) + probe(3, 3, 1.0, find_size=[W, 98]) + find_bores(3, 1.5) + [
    op(3, "face", "Face to 8 (1.3)", tool=24, stepdown=0.65),
    op(3, "adaptive", "Rough the sides and the gap to Z5 (separates the sliders)", tool=24, bottom=5.02, stepdown=3.0, stepover=0.08, leave=0.3, leave_floor=0.2),
    op(3, "contour", "Finish the outline 8 to 5, one full-height pass", tool=24, side="outside", bottom=5.02, stepdown=10, lead=2, ramp_angle=90),
    op(3, "chamfer", "Ingot chamfer 2 x 45 round the top", tool=1, chamfer=2.0, side="outside", tip_offset=0.95),
] + out(3, "slider-op2")

# the default engraving: its own program, run straight after op 2 without unclamping (swap it for a name or logo)
S += [setup("Slider op2 engrave", 4, "Brass C360", follows="Slider op2 top", z=LEDGE_Z - 0.2, station_x=FIXED_R + SW / 2, fixtures=jaws(SW, -1), model="DMC2-twostation-slider", max_feed=BRASS_LINK, wcs=2)]
S += [tools(4, BRASS)[0],
      op(4, "trace", "Engrave the crosshatch 0.4 deep (chamfer mill tip)", tool=1, sketches=[9, 10], depth=0.4, stepdown=0.4)] + out(4, "slider-engrave") + cycle(2, "slider-cycle")
S += [{"tool": "save_document", "args": {"path": "slider-cam.json"}}]
json.dump(S, open("cam.json", "w"), indent=1)
print(len(S), "steps")
