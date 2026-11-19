"""Ingot slider CAM: op 1 and op 2 for the 6061 bases and the C360 sliders, two parts per blank, as single
programs and as two-station cycles (rCAD jobs). Writes cam.json for `rcad run`.

PROVISIONAL (2026-11-18): planned on the nominal bars Jonas bought (6061 1/2 x 1 in, C360 3/8 x 1 in on 1.5 mm
shims); set BAR_* to the measured sizes and re-run.

No vertical wall is cut in two ops: op 1 finishes the walls as high as the jaws allow (down to Z5.0 beside them);
op 2 roughs the band above those walls off without touching them (0.3 radial allowance, down to exactly the
walls' top) and the top chamfer (base rail-top 1.7, slider ingot 2.4) takes the rest, its lower edge 0.2 below the
top of the op 1 wall, so the seam lies inside the chamfer face.

Z0 is the vise's built-in ledges (rcnc's Vise reference) except brass op 1, which lies on the shims and takes Z
from the probed top (which op 1 faces off). The jaws grip 4.7 above the ledges; beside them the tool stays 0.3
over their tops (fixture_clearance).
"""
import json

# ---- the bar (nominal until Jonas measures it) ----
BAR_AL = (25.4, 12.7)       # width, height: 6061 1/2 x 1 in
BAR_BR = (25.4, 9.5)        # C360 3/8 x 1 in
SHIM_BR = 1.5               # brass op 1 lies on two shim strips this thick (op 1's walls then reach Z5.8 in op 2)
BLANK_L = 100.0             # sawn 99.5 +- 0.5; op 1 finishes the parts' outer ends 98 apart
FACE_AL, FACE_BR = 0.2, 0.2  # op 1 facing of the bar top (the parts' bottoms)
BASE_T, SLIDER_T = 9.0, 8.0
JAW_LIMIT = 5.0             # lowest the tool goes beside the jaws (their top 4.7 + 0.3)
RAIL_CH, INGOT_CH = 1.7, 2.4  # the chamfers that hide the seam (build.py BCH / SCH)

# ---- the spindle (guidelines.md, 2026-11-18: F004 = 150, 9k is the strong speed) ----
# the power available falls above 9000 rpm as (9000 / rpm)^0.76: caution 850 W at 9k, 684 at 12k, 403 at 24k
M = {"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "link_feed": 5000, "rapid_z": 5000,
     "tool_changer": False, "spin_up": 4, "spindle_power": 2600, "spindle_efficiency": 0.43, "spindle_rated_rpm": 9000,
     "spindle_above_rated": 0.76, "spindle_never_exceed": 0.85, "spindle_caution": 0.76}
CAUTION = {9000: 850, 12000: 684, 18000: 501, 24000: 403}

JAW_T, JAW_H, JAW_L = 12.7, 4.7, 95.5
# stations: the vise's centre block is the fixed jaw of both. Left station (op 1, G54): fixed jaw on its +X side;
# right station (op 2 and the engraving, G55): fixed jaw on its -X side. The op 1 setup leads an rCAD job (`with`):
# export_gcode on it posts both stations as one program, `stations` posts op 1 alone.
FIXED_L, FIXED_R, TABLE_X, LEDGE_Z = -7.8, 25.5, 47.8, 23.195
# two bores through the carrier in the 8 mm gap between the parts: op 2 finds them (Find bores) after Find stock.
# Symmetric about X0, so the flip can go either way.
CARRIER = [[-5.0, 0.0], [5.0, 0.0]]

# aluminium: rough at 9k to the power budget, top-surface roughing (facing) and finishing at 12k, chamfer at 24k
AL = {"rough_rpm": 9000, "face_rpm": 12000, "fin_rpm": 12000, "ch_rpm": 24000, "feed": 3000, "fin_feed": 2000, "plunge": 400,
      "ch_feed": 1850, "target": 650}
# brass: no measurements for C360 yet and the high-helix DLC cutter can pull into it: kept mild at 18k (the speed the
# earlier plan used), feeds as before, and the power budget well under its caution (501 W at 18k)
BR = {"rough_rpm": 18000, "face_rpm": 18000, "fin_rpm": 18000, "ch_rpm": 18000, "feed": 1350, "fin_feed": 1000, "plunge": 250,
      "ch_feed": 1000, "target": 300}


def jaws(w, fixed_side, z0=0.0, extra=()):
    """Jaw boxes (work coordinates) for a part or blank w wide centred on X0, gripping from z0 to z0 + 4.7."""
    fx, mv = "fixed jaw (centre block)", "moving jaw"
    left, right = (fx, mv) if fixed_side < 0 else (mv, fx)
    return [{"name": f"{left} (-X)", "min": [-w / 2 - JAW_T, -JAW_L / 2, z0], "max": [-w / 2, JAW_L / 2, z0 + JAW_H]},
            {"name": f"{right} (+X)", "min": [w / 2, -JAW_L / 2, z0], "max": [w / 2 + JAW_T, JAW_L / 2, z0 + JAW_H]}] + list(extra)


def tools(c, f, chamfer=True):
    """The chamfer mill first: cam_op's tool takes an id or a number and ids win, so T1 must be id 1."""
    t = []
    if chamfer:
        t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "ALUMINUM", "name": "ALUMINUM chamfer 90", "number": 1, "holder": "ER16 collet chuck", "stickout": 22,
                                               "rpm": f["ch_rpm"], "feed": f["ch_feed"], "plunge_feed": 200}})
    t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "3 FLUTE DLC 1/4", "name": "3 FLUTE DLC 1/4", "number": 24, "holder": "ER16 collet chuck", "stickout": 24,
                                           "rpm": f["rough_rpm"], "feed": f["feed"], "plunge_feed": f["plunge"], "stepdown": 3, "stepover": 0.15}})
    t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "Touch probe", "name": "YIDOFENG 3D touch probe", "number": 99, "diameter": 2.0, "flute_length": 12, "holder": "ER16 collet chuck"}})
    return t


def probe(c, depth, tol):
    return [{"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": "Probe stock top", "probing": {"cycle": "z", "depth": depth, "tolerance": tol}}},
            {"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": "Probe stock centre", "probing": {"cycle": "boss", "depth": depth, "tolerance": tol}}}]


def find_bores(c, depth):
    """The two carrier bores op 1 cut: rcnc Find bores sets X/Y (and the turn, rcnc PR 18) from them."""
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
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{stem}.html", **k}},
            {"tool": "export_cam_simulation", "args": {"cam": c, "path": f"{stem}-sim.html", "resolution": 500, **k}}]


def cycle(c, stem):
    """The whole job c leads: both stations in one program, simulation, sheet and playback."""
    return [{"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim.png", "resolution": 1200}},
            {"tool": "export_gcode", "args": {"cam": c, "path": f"{stem}.nc"}},
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{stem}.html"}},
            {"tool": "export_cam_simulation", "args": {"cam": c, "path": f"{stem}-sim.html", "resolution": 700}}]


def setup(name, studio, stock=None, follows=None, orient="top", origin="part_bottom_center", station_x=0.0, fixtures=(), z=LEDGE_Z,
          model="DMC2", wcs=1, with_=(), zref="Vise"):
    a = {"post": "rcnc", "machine": dict(M, model=model), "max_rpm": 24000, "rapid_rate": 5000, "clearance": 5, "retract": 2, "fixture_clearance": 0.3,
         "work_offset": wcs, "stay_down": True, "probe": {"z_reference": zref}, "name": name, "studio": studio, "orient": orient, "origin": origin,
         "on_table": [station_x - TABLE_X, 0, z], "fixtures": list(fixtures)}
    if stock:
        a["stock"] = stock
    if follows:
        a["follows"] = follows
    if with_:
        a["with"] = list(with_)
    return {"tool": "create_cam", "args": a}


def rough(c, f, name, bottom, stepdown, rpm=None, leave_floor=0.2, **kw):
    """Adaptive roughing from the power budget: the stepover is picked so the steady cut takes the target at the
    feed, the feed drops only where the engagement rises, capped at the caution at that speed; climb."""
    rpm = rpm or f["rough_rpm"]
    return op(c, "adaptive", name, tool=24, bottom=bottom, stepdown=stepdown, leave=0.3, leave_floor=leave_floor, rpm=rpm, feed=f["feed"],
              chip_thinning=True, power_target=f["target"], power_cap=CAUTION[rpm], feed_schedule=True, **kw)


S = [{"tool": "open_document", "args": {"path": "slider.json"}},
     {"tool": "import_tool_library", "args": {"path": "../tools/fusion-library.csv", "material": "carbide"}}]

# ======================= 6061 bases =======================
W, H = BAR_AL
BW = 25.0
top1 = H - FACE_AL                  # op 1: the bases' bottoms after facing
wall_top = top1 - JAW_LIMIT         # op 2: how high op 1's walls reach
assert BASE_T - RAIL_CH <= wall_top - 0.2 + 1e-9, f"rail chamfer lower edge Z{BASE_T - RAIL_CH} not 0.2 under the op 1 wall top Z{wall_top}"
S += [setup("Base op1 bottom", 3, stock={"size": [W, BLANK_L, H], "bottom": H - FACE_AL - BASE_T, "material": "6061-T6"}, orient="bottom",
            origin="stock_bottom_center", station_x=FIXED_L - W / 2, fixtures=jaws(W, +1), model="DMC2-twostation-base", with_=["Base op2 top"])]
S += tools(0, AL) + probe(0, 3, 1.5) + [
    op(0, "face", f"Face the bar top (the bases' bottoms) {FACE_AL}", tool=24, stepdown=FACE_AL, stepover=0.4, rpm=AL["face_rpm"], feed=AL["feed"]),
    rough(0, AL, f"Rough round both bases, the gap and the ends to Z{JAW_LIMIT:g} (laps, power budget)", bottom=JAW_LIMIT,
          stepdown=(top1 - JAW_LIMIT) / 2, leave_floor=0.0),
    op(0, "contour", f"Finish the walls in one full-height pass, down to Z{JAW_LIMIT:g} (op 2 cuts no wall below the chamfer)", tool=24,
       side="outside", bottom=JAW_LIMIT, stepdown=20, lead=2, rpm=AL["fin_rpm"], feed=AL["fin_feed"]),
    op(0, "bore", "Two locating bores 7.5 through the carrier between the bases (op 2 finds them)", tool=24, points=CARRIER, thread_diameter=7.5,
       bottom=-0.3, rpm=AL["rough_rpm"], feed=1500),
    op(0, "chamfer", "Edge break 0.3 the bottom outline (op 2 sits it on the 1.9 ledges)", tool=1, chamfer=0.3, side="outside", tip_offset=1.0),
] + out(0, "base-op1", ["Base op1 bottom"])

# op 2: flipped, finished bottom on the ledges, op 1's walls in (protected) jaws
S += [setup("Base op2 top", 3, follows="Base op1 bottom", z=LEDGE_Z - FACE_AL, station_x=FIXED_R + BW / 2, fixtures=jaws(BW, -1),
            model="DMC2-twostation-base", wcs=2)]
S += tools(1, AL) + probe(1, 3, 1.5) + find_bores(1, 1.0) + [
    op(1, "face", f"Face the top down to Z{BASE_T:g} ({top1 - BASE_T:g}, two levels)", tool=24, stepdown=(top1 - BASE_T) / 2, stepover=0.2,
       rpm=AL["face_rpm"], feed=AL["feed"]),
    rough(1, AL, "Rough the channel to Z6", bottom=6.0, stepdown=3.0),
    op(1, "parallel", "Finish the channel floor (Z6), passes along the slide (Y)", tool=24, angle=90, stepover=0.4, slope_max=2, bottom=6.0, leave=0.0, rest=True,
       rpm=AL["fin_rpm"], feed=AL["fin_feed"]),
    op(1, "bore", "Magnet pockets 8.05 x 3.15 (helical, finish floor)", tool=24, rpm=AL["fin_rpm"], feed=1500),
    rough(1, AL, f"Rough the band above op 1's walls off to Z{wall_top - 0.2:g}, 0.2 under their top Z{wall_top:g} (no web left), 0.3 off them (separates the bases)",
          bottom=wall_top - 0.2, stepdown=BASE_T - wall_top + 0.2, leave_floor=0.0),
    op(1, "trace", "Finish the rails' channel walls, Z9 to Z6, one pass (traces inside the channel sketch, straight through the cleared gap)", tool=24,
       sketches=[14], side="inside", top=BASE_T, depth=BASE_T - 6.0, stepdown=BASE_T - 6.0, rpm=AL["fin_rpm"], feed=AL["fin_feed"]),
    op(1, "deburr", f"Chamfer {RAIL_CH} the rail tops outside (takes the 0.3 left on the band and hides the op 1 / op 2 seam)", tool=1,
       chamfer=RAIL_CH, tip_offset=1.0, edges=[[s * BW / 2, cy, BASE_T] for s in (1, -1) for cy in (-26.5, 26.5)]),
    op(1, "deburr", "Edge break 0.3: channel edges, floor ends, pocket rims", tool=1, chamfer=0.3, tip_offset=0.7,
       edges=[[s * 10.0, cy, BASE_T] for s in (1, -1) for cy in (-26.5, 26.5)] + [[0, cy + s * 22.5, 6.0] for s in (1, -1) for cy in (-26.5, 26.5)]
       + [[4.025, cy + m, 6.0] for m in (-15, 0, 15) for cy in (-26.5, 26.5)]),
] + out(1, "base-op2") + cycle(0, "base-cycle")

# ======================= C360 sliders =======================
W, H = BAR_BR
SW = 19.6
top1 = H - FACE_BR                  # op 1 (blank coordinates: Z0 is the blank's bottom on the shims)
low1 = JAW_LIMIT - SHIM_BR          # op 1's lowest beside the jaws, in those coordinates
wall_top = top1 - low1              # op 2: how high op 1's walls reach
assert SLIDER_T - INGOT_CH <= wall_top - 0.2 + 1e-9, f"ingot chamfer lower edge Z{SLIDER_T - INGOT_CH} not 0.2 under the op 1 wall top Z{wall_top}"
shims = [{"name": f"{SHIM_BR} mm shim strip (25 x 39) under the {e} slider, op 1 only", "min": [-12.5, lo, -SHIM_BR], "max": [12.5, lo + 39, 0]}
         for e, lo in (("-Y", -45), ("+Y", 6))]
S += [setup("Slider op1 bottom", 4, stock={"size": [W, BLANK_L, H], "bottom": H - FACE_BR - SLIDER_T, "material": "Brass C360"}, orient="bottom",
            origin="stock_bottom_center", station_x=FIXED_L - W / 2, fixtures=jaws(W, +1, z0=-SHIM_BR, extra=shims), model="DMC2-twostation-slider",
            with_=["Slider op2 top", "Slider op2 engrave"], zref="")]
S += tools(2, BR) + probe(2, 3, 1.5) + [
    op(2, "face", f"Face the bar top (the sliders' bottoms) {FACE_BR}, passes along the slide (Y)", tool=24, stepdown=FACE_BR, angle=90, stepover=0.4),
    rough(2, BR, f"Rough round both sliders, the gap and the ends to Z{low1:g} (laps, power budget)", bottom=low1, stepdown=(top1 - low1) / 3,
          leave_floor=0.0),
    op(2, "contour", f"Finish the walls in one full-height pass, down to Z{low1:g} (op 2 cuts no wall below the chamfer)", tool=24, side="outside",
       bottom=low1, stepdown=20, lead=2, feed=BR["fin_feed"]),
    op(2, "bore", "Magnet pockets 8.05 x 3.15 (helical, finish floor)", tool=24, feed=BR["fin_feed"]),
    op(2, "bore", "Two locating bores 7.5 through the carrier between the sliders (op 2 finds them)", tool=24, points=CARRIER, thread_diameter=7.5,
       bottom=-0.3, feed=BR["fin_feed"]),
    op(2, "chamfer", "Edge break 0.3: bottom outline and pocket rims", tool=1, chamfer=0.3, side="both", tip_offset=1.0),
] + out(2, "slider-op1", ["Slider op1 bottom"])

S += [setup("Slider op2 top", 4, follows="Slider op1 bottom", z=LEDGE_Z - FACE_BR, station_x=FIXED_R + SW / 2, fixtures=jaws(SW, -1),
            model="DMC2-twostation-slider", wcs=2)]
S += tools(3, BR) + probe(3, 3, 1.5) + find_bores(3, 1.5) + [
    op(3, "face", f"Face the top down to Z{SLIDER_T:g} ({top1 - SLIDER_T:g})", tool=24, stepdown=top1 - SLIDER_T, stepover=0.4),
    rough(3, BR, f"Rough the band above op 1's walls off to Z{wall_top - 0.2:g}, 0.2 under their top Z{wall_top:g} (no web left), 0.3 off them (separates the sliders)",
          bottom=wall_top - 0.2, stepdown=SLIDER_T - wall_top + 0.2, leave_floor=0.0),
    op(3, "chamfer", f"Ingot chamfer {INGOT_CH} x 45 round the top (takes the 0.3 left on the band, hides the seam; tip at Z{SLIDER_T - INGOT_CH - 0.58:.2f})",
       tool=1, chamfer=INGOT_CH, side="outside", tip_offset=0.58),
] + out(3, "slider-op2")

# the default engraving: its own program, run straight after op 2 without unclamping (swap it for a name or logo)
S += [setup("Slider op2 engrave", 4, follows="Slider op2 top", z=LEDGE_Z - FACE_BR, station_x=FIXED_R + SW / 2, fixtures=jaws(SW, -1),
            model="DMC2-twostation-slider", wcs=2)]
S += [tools(4, BR)[0],
      op(4, "trace", "Engrave the crosshatch 0.4 deep (chamfer mill tip)", tool=1, sketches=[9, 10], depth=0.4, stepdown=0.4)] + out(4, "slider-engrave") + cycle(2, "slider-cycle")
S += [{"tool": "save_document", "args": {"path": "slider-cam.json"}}]
json.dump(S, open("cam.json", "w"), indent=1)
print(len(S), "steps")
