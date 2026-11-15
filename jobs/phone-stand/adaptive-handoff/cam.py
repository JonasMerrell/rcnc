import json
M = {"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000, "tool_changer": False, "spin_up": 4, "spindle_power": 1500, "spindle_efficiency": 0.78, "spindle_rated_rpm": 24000}
def jaws(half, note):
    return [{"name": f"fixed vise jaw (LEFT, -X); {note}; jaws grip the bottom 4.7 mm", "min": [-half - 12.7, -47.75, 0], "max": [-half, 47.75, 4.7]},
            {"name": "moving vise jaw (right, +X)", "min": [half, -47.75, 0], "max": [half + 12.7, 47.75, 4.7]}]
def tools(c, ball=False, chamfer=False):
    t = []
    if chamfer:
        t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "ALUMINUM", "name": "ALUMINUM chamfer 90", "number": 1, "holder": "ER16 collet chuck", "stickout": 22, "rpm": 24000, "feed": 1850, "plunge_feed": 200}})
    t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "3 FLUTE DLC 1/4", "name": "3 FLUTE DLC 1/4", "number": 24, "holder": "ER16 collet chuck", "stickout": 24, "rpm": 24000, "feed": 2400, "plunge_feed": 330, "stepdown": 3, "stepover": 0.4}})
    if ball:
        t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "6mm 2 flute ball end mill", "name": "6 mm 2FL ball (carbide)", "number": 11, "material": "carbide", "holder": "ER16 collet chuck", "stickout": 21, "rpm": 24000, "feed": 3000, "plunge_feed": 600, "stepdown": 1, "stepover": 0.06}})
    t.append({"tool": "cam_tool", "args": {"cam": c, "preset": "Touch probe", "name": "YIDOFENG 3D touch probe", "number": 99, "diameter": 2.0, "flute_length": 12, "holder": "ER16 collet chuck"}})
    return t
probe = lambda c, depth=3: [{"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": "Probe stock top", "probing": {"cycle": "z", "depth": depth, "tolerance": 3.0}}},
                   {"tool": "cam_op", "args": {"cam": c, "kind": "probe", "tool": 99, "name": "Probe stock centre", "probing": {"cycle": "boss", "depth": depth, "tolerance": 3.0}}}]
def out(c, stem):
    return [{"tool": "cam_info", "args": {"cam": c}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{stem}-sim.png", "resolution": 600}},
            {"tool": "render_cam", "args": {"cam": c, "path": f"{stem}-paths.png"}},
            {"tool": "export_gcode", "args": {"cam": c, "path": f"{stem}.nc"}},
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{stem}.html"}}]
S = [{"tool": "open_document", "args": {"path": "stand.json"}},
     {"tool": "import_tool_library", "args": {"path": "fusion-library.csv", "material": "carbide"}},
     # Op 1: the underside, factory face down on the built-in parallels
     {"tool": "create_cam", "args": {"post": "rcnc", "machine": M, "max_rpm": 24000, "clearance": 5, "retract": 2, "probe": {"z_reference": "Vise"},
        "name": "Op1 underside", "orient": "bottom", "origin": "stock_bottom_center",
        "stock": {"size": [76, 76, 48], "bottom": 4.5, "material": "6061-T6"}, "on_table": [15.7, 0, 23.195],
        "fixtures": jaws(38, "the 76 x 76 slab (up to 48 thick, saw cut uneven), factory face down on the built-in parallels (Z0, the Vise reference). For 76 mm: moving jaw base one 25 mm hole further out (+X) than for the 39 mm block, jaw opened about 12 mm more")}}]
S += tools(0, chamfer=True) + probe(0, depth=10) + [
     {"tool": "cam_op", "args": {"cam": 0, "kind": "face", "tool": 24, "name": "Face the saw cut down to the underside (up to 14 mm)", "stepdown": 1.5}},
     {"tool": "cam_op", "args": {"cam": 0, "kind": "adaptive", "tool": 24, "bottom": 19.5, "stepdown": 5, "stepover": 0.12, "leave": 0.2, "leave_floor": 0.0, "name": "Rough round the plinth (adaptive: the corners in small bites)"}},
     {"tool": "cam_op", "args": {"cam": 0, "kind": "contour", "tool": 24, "side": "outside", "bottom": 19.5, "stepdown": 15, "lead": 2, "ramp_angle": 90, "name": "Finish the plinth walls, one full-height pass"}},
     {"tool": "cam_op", "args": {"cam": 0, "kind": "pocket", "tool": 24, "at": [0, 0], "stepdown": 1, "rest": False, "name": "Underside recess 2 (finishes its floor and walls)"}},
     {"tool": "cam_op", "args": {"cam": 0, "kind": "bore", "tool": 24, "breakthrough": 1.0, "name": "2x locating bore 8 (up through the groove floor)"}},
     {"tool": "cam_op", "args": {"cam": 0, "kind": "chamfer", "tool": 1, "chamfer": 0.5, "side": "both", "tip_offset": 1.0, "name": "Chamfer 0.5 underside edges"}}] + out(0, "op1-underside") + [
     {"tool": "cam_op", "args": {"cam": 0, "id": i, "suppressed": True}} for i in (4, 5, 6)] + [
     # the restart starts on the faced slab: its top is the underside, Z34
     {"tool": "edit_cam", "args": {"cam": 0, "stock": {"size": [76, 76, 34], "bottom": 4.5, "material": "6061-T6"}}},
     {"tool": "simulate_cam", "args": {"cam": 0, "path": "op1-restart-sim.png", "resolution": 600}},
     {"tool": "export_gcode", "args": {"cam": 0, "path": "op1-restart-contour.nc"}},
     {"tool": "export_setup_sheet", "args": {"cam": 0, "path": "op1-restart-contour.html"}},
     {"tool": "edit_cam", "args": {"cam": 0, "stock": {"size": [76, 76, 48], "bottom": 4.5, "material": "6061-T6"}}},
     ] + [{"tool": "cam_op", "args": {"cam": 0, "id": i, "suppressed": False}} for i in (4, 5, 6)]
# Op 2: the sculpted top, finished underside down, plinth in the jaws
S += [{"tool": "create_cam", "args": {"post": "rcnc", "machine": M, "max_rpm": 24000, "clearance": 5, "retract": 2, "probe": {"z_reference": "Vise"},
        "name": "Op2 top", "follows": "Op1 underside", "orient": "top", "origin": "part_bottom_center", "on_table": [14.7, 0, 23.195],
        "fixtures": jaws(37, "the finished plinth (74 wide) against it, underside down on the built-in parallels (Z0)")}}]
S += tools(1, ball=True) + probe(1) + [
     {"tool": "cam_op", "args": {"cam": 1, "kind": "face", "tool": 24, "name": "Face to the crest", "stepdown": 2}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "adaptive", "tool": 24, "stepdown": 3.3, "stepover": 0.22, "leave": 0.3, "leave_floor": 0.2, "chip_thinning": False, "name": "3D rough (adaptive)"}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "parallel", "tool": 24, "angle": 0, "stepover": 0.25, "leave": 1.7, "bottom": 10.15, "name": "3D semi-rough 1: T24 parallel over the rough's steps (top half)"}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "parallel", "tool": 24, "angle": 0, "stepover": 0.25, "leave": 0.3, "bottom": 10.15, "name": "3D semi-rough 2: T24 parallel down to 0.3 over the part"}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "clear", "tool": 24, "top": 10.6, "bottom": 10.0, "stepdown": 0.6, "stepover": 0.4, "leave": 0.0, "leave_floor": 0.0, "rest": False, "name": "Finish the groove floor (Z10, T24 flat, into the corners)"}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "probe", "tool": 99, "name": "Probe locating bore A", "at": [-18, -19.25], "probing": {"cycle": "bore", "on_part": True, "size": 8, "depth": 3, "tolerance": 0.1, "max_turn": 0.2}}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "probe", "tool": 99, "name": "Probe locating bore B", "at": [18, -19.25], "probing": {"cycle": "bore", "on_part": True, "size": 8, "depth": 3, "tolerance": 0.1, "max_turn": 0.2}}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "parallel", "tool": 11, "angle": 90, "stepover": 0.08, "slope_max": 65, "bottom": 9.98, "name": "Finish: parallel passes front to back"}},
     {"tool": "cam_op", "args": {"cam": 1, "kind": "waterline", "tool": 11, "stepdown": 0.45, "slope_min": 40, "bottom": 9.98, "name": "Finish: waterline on the steep faces"}}] + out(1, "op2-top") + [
     {"tool": "save_document", "args": {"path": "stand-cam.json"}}]
json.dump(S, open("cam.json", "w"), indent=1)
