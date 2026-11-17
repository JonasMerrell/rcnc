"""Stick-out gauge CAM, op 2: the gauge's bottom side. Writes cam2.json for `rcad run`; it opens gauge-cam.json,
which cam.json (op 1a and op 1) saves.

Flipped about Y: op 1's finished top down on the vise's ledges (Z0 = the Vise reference), the jaws on the
finished 23 mm walls, the end lugs standing up past the jaw ends with op 1's Ø8 bores in them. rcnc Find bores
sets X/Y from those bores before any cutting.

rCAD only lets a setup follow another made from the same Part Studio, and op 1's (studio 3) keeps the carrier and
lugs as part, so op 2 machines studio 4 (the gauge alone) from a box: the bar 19 tall, as op 1 faced it. That box
has more stock than there really is (op 1 left the lugs 8 thick and the side strips gone down to Z5 in its
frame), so op 2's simulated loads are on the high side: real cuts there are lighter or in air.
"""
import json

M = {"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000, "tool_changer": False, "spin_up": 4,
     "spindle_power": 1500, "spindle_efficiency": 0.78, "spindle_rated_rpm": 24000}
JAW_T, JAW_H, JAW_L = 12.7, 4.7, 95.5
FIXED_R, TABLE_X, LEDGE_Z = 25.5, 47.8, 23.195
W, BAR_L = 23.0, 120.7


def jaws(w):
    return [{"name": "fixed jaw (centre block, -X)", "min": [-w / 2 - JAW_T, -JAW_L / 2, 0], "max": [-w / 2, JAW_L / 2, JAW_H]},
            {"name": "moving jaw (+X)", "min": [w / 2, -JAW_L / 2, 0], "max": [w / 2 + JAW_T, JAW_L / 2, JAW_H]}]


def op(c, kind, name, **a):
    a.update({"cam": c, "kind": kind, "name": name})
    return {"tool": "cam_op", "args": a}


C = 2
S = [{"tool": "open_document", "args": {"path": "gauge-cam.json"}},
     {"tool": "create_cam", "args": {"post": "rcnc", "machine": M, "max_rpm": 24000, "clearance": 5, "retract": 2, "fixture_clearance": 0.3, "probe": {"z_reference": "Vise"},
                                     "name": "Op 2 bottom", "studio": 3, "bodies": [[2, 0]], "follows": "Op 1 top", "orient": "bottom", "origin": "part_bottom_center",
                                     "on_table": [FIXED_R + W / 2 - TABLE_X, 0, LEDGE_Z], "fixtures": jaws(W)}}]
# follows op 1: its stock (what op 1 really left: lugs 8 thick with their bores, the carrier) and its tools
S += [op(C, "probe", f"Probe lug bore {'AB'[i]}", tool=99, at=xy,
         probing={"cycle": "bore", "on_part": True, "size": 8.0, "depth": 3.0, "tolerance": 0.1, "max_turn": 0.2}) for i, xy in enumerate([[0, -52.0], [0, 52.0]])]
S += [
    op(C, "face", "Face the 3 mm carrier off (Z19 to Z16): finishes the gauge's bottom", tool=24, top=19.0, stepdown=1.0),
    op(C, "trace", "Rough the bottom slots and their end reliefs, 0.2 left on the walls (1 mm levels, F1000; floors to depth)", tool=24, sketches=[5], side="inside", top=16.0, depth=6.0, stepdown=1.0, leave=0.2, feed=1000, plunge_feed=250),
    op(C, "adaptive", "Rough the lugs and the side strips off down to Z5", tool=24, bottom=5.0, stepdown=5.5, stepover=0.1, leave=0.3, leave_floor=0.0, rest=True),
    op(C, "trace", "Finish the slot walls and the relief stop walls, one full-depth 6 mm pass round their outline", tool=24, sketches=[5], side="inside", top=16.0, depth=6.0, stepdown=6.0, leave=0.0),
    op(C, "contour", "Finish the walls and end faces Z16 to Z10.5, one full-height pass (meets op 1's finish; seam mid -X side)", tool=24, side="outside", top=16.0, bottom=10.5, stepdown=7, seam=[-20, 0]),
    op(C, "trace", "Engrave the bottom numbers 0.3 deep", tool=1, sketches=[10], depth=0.3, stepdown=0.3),
    op(C, "chamfer", "Edge break 0.3 on every edge of the bottom face: outline, slots, reliefs", tool=1, chamfer=0.3, side="both", tip_offset=1.0),
    op(C, "deburr", "Edge break 0.3 on the four vertical corners", tool=1, chamfer=0.3, tip_offset=1.0,
       edges=[[sx * W / 2, sy * 44.0, 8.0] for sx in (1, -1) for sy in (1, -1)]),
]
S += [{"tool": "cam_info", "args": {"cam": C}},
      {"tool": "simulate_cam", "args": {"cam": C, "path": "op2-bottom-sim.png", "resolution": 900}},
      {"tool": "simulate_cam", "args": {"cam": C, "path": "op2-bottom-sim-top.png", "view": "top", "resolution": 900}},
      {"tool": "render_cam", "args": {"cam": C, "path": "op2-bottom-paths.png"}},
      {"tool": "export_gcode", "args": {"cam": C, "path": "op2-bottom.nc"}},
      {"tool": "export_setup_sheet", "args": {"cam": C, "path": "op2-bottom.html"}},
      {"tool": "save_document", "args": {"path": "gauge-cam2.json"}}]
json.dump(S, open("cam2.json", "w"), indent=1)
print(len(S), "steps")
