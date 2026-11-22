"""CAM for the Eclipse car (approved surface) and the thick-rail base, from eclipse-car.json. Programs in
eclipse-car/ (car) and eclipse-base-thick/ (base). Writes cam.json for `rcad run`.

Car op 1 = the ingot slider's brass op 1 with the car outline: the blank on the 1.67 shims, Z from the probed top
(op 1 faces it), walls in one full-height pass down to Z3.33 (car z 6.47: every vertical wall of the car, the
highest is the tail lamp panel at 6.50), magnet pockets, carrier bores, 0.3 edge break. Car op 2 stands on 1.67
shims too (grip 3.0) and takes Z from the vise like every op 2 (the shims are in the model), so only op 1 of a
cycle takes Z from Find stock. Light roughing (100 W, cap 120) while the band still joins the cars, a light rough that separates
them, then T44 (tapered ball R1) semi-finish, finish (parallel + waterline 0.07 = 0.035 D) and T1 edge break.
The base is the ingot base's CAM with the 17.7 channel."""
import json, sys
src = open("/home/jonas/dmc2/slider/cam.py").read()
exec(src[:src.index("S = [{\"tool\": \"open_document\"")])

HERE = "/home/jonas/dmc2/eclipse-car"
CARD, BASED = "eclipse-car", "eclipse-base-thick"
ids = json.load(open(f"{HERE}/sketch_ids.json"))
# T44 = SpeTool W01004 tapered ball (Jonas's spec): R1.0 tip, 2.24 deg per side, 2 flutes, 15 flute, 1/8 shank,
# 38.1 overall, TiAlN carbide. ASSUMED (not confirmed by Jonas): 18 mm stick-out (flute + 3; 20 in the ER16 collet),
# F1500 at 24 000 rpm (0.031 mm/tooth nominal; at the 0.07 finishing stepover the real chip is a few microns),
# plunge F750 (entries come down onto at most 0.3 of brass; at F300 they took 4.7 min of op 2).
BALL_FEED, BALL_RPM, BALL_OUT = 1500, 24000, 18
BALL = {"kind": "tapered_ball", "name": "SpeTool W01004 tapered ball R1 2.24 deg (TiAlN)", "number": 44, "diameter": 3.175, "corner_radius": 1.0,
        "angle": 2.24, "flutes": 2, "flute_length": 15, "stickout": BALL_OUT, "holder": "ER16 collet chuck",
        "rpm": BALL_RPM, "feed": BALL_FEED, "plunge_feed": 750}
BT = 44
CAR_T, CAR_W, SURF_LO = 9.5, 17.3, 3.9    # nothing shaped below z 3.9
PAIR = 26.5


# the export gate's stock simulation: 1000 cells along the longer side (default 500). The two-station cycle's
# stock spans both stations, so at 500 its cells were twice as coarse and flagged 0.03-0.05 at a tail-lip corner
# that the single op 2 (and a 0.025 mm z-map of its T44 moves) shows clean
RES = 1000


def outs(c, folder, stem, only=None):
    k = {"stations": only} if only else {}
    return [{"tool": "cam_info", "args": {"cam": c}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{folder}/{stem}-sim.png", "resolution": 500, **k}},
            {"tool": "simulate_cam", "args": {"cam": c, "path": f"{folder}/{stem}-sim-top.png", "view": "top", "resolution": 500, **k}},
            {"tool": "render_cam", "args": {"cam": c, "path": f"{folder}/{stem}-paths.png"}},
            {"tool": "export_gcode", "args": {"cam": c, "path": f"{folder}/{stem}.nc", "resolution": RES, **k}},
            {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{folder}/{stem}.html", **k}}]


def cycles(c, folder, stem, first, last):
    S = [{"tool": "simulate_cam", "args": {"cam": c, "path": f"{folder}/{stem}-cycle-sim.png", "resolution": 500}},
         {"tool": "export_gcode", "args": {"cam": c, "path": f"{folder}/{stem}-cycle.nc", "resolution": RES}},
         {"tool": "export_setup_sheet", "args": {"cam": c, "path": f"{folder}/{stem}-cycle.html"}}]
    for which, st in (("first", first), ("last", last)):
        S += [{"tool": "export_gcode", "args": {"cam": c, "stations": st, "path": f"{folder}/{stem}-{which}.nc", "resolution": RES}},
              {"tool": "export_setup_sheet", "args": {"cam": c, "stations": st, "path": f"{folder}/{stem}-{which}.html"}}]
    return S


S = [{"tool": "open_document", "args": {"path": f"{HERE}/eclipse-car.json"}},
     {"tool": "import_tool_library", "args": {"path": "/home/jonas/dmc2/tools/fusion-library.csv", "material": "carbide"}}]

# ======================= the car (C360) =======================
W, Hb = BAR_BR
top1 = Hb - FACE_BR                 # op 1 (blank coordinates, Z0 = the blank's bottom on the shims): the cars' bottoms
low1 = JAW_LIMIT - SHIM_BR          # op 1's lowest beside the jaws (3.33)
wall_top = top1 - low1              # car z op 1's walls reach (6.47)
shims1 = [{"name": f"{SHIM_BR} mm shim strip (25 x 39) under the {e} car, op 1", "min": [-12.5, lo, -SHIM_BR], "max": [12.5, lo + 39, 0]}
          for e, lo in (("-Y", -45), ("+Y", 6))]
S += [setup("Eclipse car op1 bottom", 1, stock={"size": [W, BLANK_L, Hb], "bottom": Hb - FACE_BR - CAR_T, "material": "Brass C360"}, orient="bottom",
            origin="stock_bottom_center", station_x=FIXED_L - W / 2, fixtures=jaws(W, +1, z0=-SHIM_BR, extra=shims1), model="DMC2-twostation-slider",
            with_=["Eclipse car op2 top"], zref="")]
S += tools(0, BR) + probe(0, 3, 1.0) + [
    op(0, "face", f"Face the bar top (the cars' bottoms) {FACE_BR}, passes along the slide (Y)", tool=24, stepdown=FACE_BR, angle=90, stepover=0.4),
    rough(0, BR, f"Rough round both cars, the gap and the ends to Z{low1:g} (laps, power budget, capped at 400 W: the jaws hold only {4.7 - SHIM_BR:.1f} of the bar)",
          bottom=low1, stepdown=(top1 - low1) / 4, leave_floor=0.0, cap=400),
    op(0, "contour", f"Finish the walls in one full-height pass, down to Z{low1:g} (car z {wall_top:.2f}: every vertical wall of the car)", tool=24, side="outside",
       bottom=low1, stepdown=20, lead=2, feed=BR["fin_feed"]),
    op(0, "bore", "Magnet pockets 8.05 x 3.15 (helical, finish floor)", tool=24, feed=BR["fin_feed"]),
    op(0, "bore", "Two locating bores 7.5 through the carrier between the cars (op 2 finds them)", tool=24, points=CARRIER, thread_diameter=7.5,
       bottom=-0.3, feed=BR["fin_feed"]),
    op(0, "chamfer", "Edge break 0.3: bottom outline and pocket rims", tool=1, chamfer=0.3, side="both", tip_offset=1.0),
] + outs(0, CARD, "eclipse-car-op1", ["Eclipse car op1 bottom"])

# op 2: flipped onto 1.67 shims narrower than the car (the jaws close on the car itself), Z from the vise
SHIM2_W = 16.0
shims2 = [{"name": f"{SHIM_BR} mm shim strip ({SHIM2_W:g} x 39) under the {e} car, op 2", "min": [-SHIM2_W / 2, lo, 0], "max": [SHIM2_W / 2, lo + 39, SHIM_BR]}
          for e, lo in (("-Y", -45), ("+Y", 6))]
Z2 = SHIM_BR                        # op 2 work Z = car z + 1.67
S += [setup("Eclipse car op2 top", 1, follows="Eclipse car op1 bottom", z=LEDGE_Z - FACE_BR, station_x=FIXED_R + CAR_W / 2, origin="model",
            fixtures=jaws(CAR_W, -1, extra=shims2), model="DMC2-twostation-slider", wcs=2)]
FLOOR = SURF_LO + Z2 - 0.05         # the ball never goes below car z 3.85 (op 1's walls below 3.9 stay untouched)
S += tools(1, BR) + [{"tool": "cam_tool", "args": dict(BALL, cam=1)}] + probe(1, 3, 1.5) + find_bores(1, 1.0) + [
    op(1, "face", f"Face the top down to the roof, car z {CAR_T:g} ({top1 - CAR_T:g})", tool=24, stepdown=top1 - CAR_T, stepover=0.4),
    # light throughout (Jonas, 2026-11-22: op 2 stands on the shims, only 3.0 of the car in the jaws): every T24
    # cut in op 2 at about 100 W, capped at 120 W
    rough(1, BR, f"Rough while the band still joins the cars, light (100 W, cap 120): down to car z {wall_top + 0.05:.2f} (0.3 left)", bottom=wall_top + 0.05 + Z2,
          stepdown=1.0, leave_floor=0.2, target=100, cap=120),
    rough(1, BR, f"Rough the rest down to car z {SURF_LO:g} (separates the cars, light: 100 W, cap 120)", bottom=SURF_LO + Z2,
          stepdown=1.0, leave_floor=0.0, target=100, cap=120),
    op(1, "parallel", "Semi-finish the car tops with T44 (0.25 stepover, leaves 0.05)", tool=BT, angle=90, stepover=0.25, leave=0.05, bottom=FLOOR),
    op(1, "parallel", "Finish the car tops with T44 along the car (0.07 stepover = 0.035 D), slopes up to 45 deg", tool=BT, angle=90, stepover=0.07, leave=0.0,
       bottom=FLOOR, slope_max=45),
    # waterline: the only finish that takes the ball outside the outline, which the tail needs (the lamp panel drops
    # from the lip 0.9 mm in from the end); rCAD's waterline needed the chord fix (branch waterline-chords): it
    # gouged this mesh up to 0.76 next to the pillars
    op(1, "waterline", f"Finish the steep faces (shoulders, glass, nose, tail lamp panel) with T44, 0.069 apart, down to car z {SURF_LO:g}",
       tool=BT, stepdown=0.069, slope_min=40, bottom=FLOOR),
    # pencil: the creases the 0.07 passes straddle (hood shut line and body creases as V grooves, glass recess edges,
    # mirror bases): T44 along every concave corner a 4 mm ball would not reach (rCAD wants the reference over the 3.175 shank)
    op(1, "pencil", "Pencil the creases with T44 (hood shut line, glass edges, mirror bases): where a 4 mm ball would not reach",
       tool=BT, ref_diameter=4.0, stepover=0.07, leave=0.0, bottom=FLOOR),
    # T1 edge break: the tail lip (deburr, picked) and the straight nose top edge at z 3.9 (a trace 0.2 deep
    # along the edge: the cone's 45 deg side breaks it). rCAD's deburr, picked at the ends, ran on round the smooth
    # R4.8 / R2.4 corners onto the rising fenders and gouged them 0.07-0.1; the corners are left to the ball.
    op(1, "deburr", "Edge break 0.2: the tail lip (T1)", tool=1, chamfer=0.2, tip_offset=0.3, edge_angle=70,
       edges=[[0, cy - 21.6, 6.96 + Z2] for cy in (-PAIR, PAIR)]),
    # (the tail end's 3.9 edge is only 0.2 from the bumper slope and 0.9 from the lamp panel: the cone hits them, so it
    # is left to the ball's passes over it)
    op(1, "trace", "Edge break 0.2: the straight nose top edge at car z 3.9 (T1 tip on the edge, 0.2 deep)", tool=1,
       sketches=[ids["car_end_edges"]], top=SURF_LO + Z2, depth=0.2, stepdown=0.2),
] + outs(1, CARD, "eclipse-car-op2") + cycles(0, CARD, "eclipse-car", ["Eclipse car op1 bottom"], ["Eclipse car op2 top"])

# ======================= the thick-rail base (6061): the ingot base's CAM, channel 17.7 =======================
W, H = BAR_AL
BW, CH_W = 25.0, 17.7
top1 = H - FACE_AL
low_al = JAW_LIMIT - SHIM_AL
wall_top = top1 - low_al
assert BASE_T - RAIL_CH <= wall_top - 0.2 + 1e-9
S += [setup("Thick base op1 bottom", 3, stock={"size": [W, BLANK_L, H], "bottom": H - FACE_AL - BASE_T, "material": "6061-T6"}, orient="bottom",
            origin="stock_bottom_center", station_x=FIXED_L - W / 2, fixtures=jaws(W, +1), model="DMC2-twostation-base",
            with_=["Thick base op2 top"], zref="Vise")]
S += tools(2, AL) + probe(2, 3, 1.0) + [
    op(2, "face", f"Face the bar top (the bases' bottoms) {FACE_AL}", tool=24, stepdown=FACE_AL, stepover=0.4, rpm=AL["face_rpm"], feed=AL["feed"]),
    rough(2, AL, f"Rough round both bases, the gap and the ends to Z{low_al:g} (laps, power budget)", bottom=low_al,
          stepdown=(top1 - low_al) / 2, leave_floor=0.0),
    op(2, "contour", f"Finish the walls in one full-height pass, down to Z{low_al:g} (op 2 cuts no wall below the chamfer)", tool=24,
       side="outside", bottom=low_al, stepdown=20, lead=2, rpm=AL["fin_rpm"], feed=AL["fin_feed"]),
    op(2, "bore", "Two locating bores 7.5 through the carrier between the bases (op 2 finds them)", tool=24, points=CARRIER, thread_diameter=7.5,
       bottom=-0.3, rpm=AL["rough_rpm"], feed=1500),
    op(2, "chamfer", "Edge break 0.3 the bottom outline (op 2 sits it on the 1.9 ledges)", tool=1, chamfer=0.3, side="outside", tip_offset=1.0),
] + outs(2, BASED, "eclipse-base-thick-op1", ["Thick base op1 bottom"])
S += [setup("Thick base op2 top", 3, follows="Thick base op1 bottom", z=LEDGE_Z - FACE_AL, station_x=FIXED_R + BW / 2, fixtures=jaws(BW, -1),
            model="DMC2-twostation-base", wcs=2)]
S += tools(3, AL) + probe(3, 3, 1.5) + find_bores(3, 1.0) + [
    op(3, "face", f"Face the top down to Z{BASE_T:g} ({top1 - BASE_T:g}, two levels)", tool=24, stepdown=(top1 - BASE_T) / 2, stepover=0.2,
       rpm=AL["face_rpm"], feed=AL["feed"]),
    rough(3, AL, f"Rough the channel (17.7 wide) to Z{CH_FLOOR:g}", bottom=CH_FLOOR, stepdown=3.0),
    op(3, "parallel", f"Finish the channel floor (Z{CH_FLOOR:g}), passes along the slide (Y)", tool=24, angle=90, stepover=0.4, slope_max=2, bottom=CH_FLOOR, leave=0.0, rest=True,
       rpm=AL["fin_rpm"], feed=AL["fin_feed"]),
    op(3, "bore", "Magnet pockets 8.05 x 3.15 (helical, finish floor)", tool=24, rpm=AL["fin_rpm"], feed=1500),
    rough(3, AL, f"Rough the band above op 1's walls off to Z{wall_top - 0.2:g}, 0.2 under their top Z{wall_top:g} (no web left), 0.3 off them (separates the bases)",
          bottom=wall_top - 0.2, stepdown=BASE_T - wall_top + 0.2, leave_floor=0.0, target=120, cap=150),
    op(3, "trace", f"Finish the rails' channel walls (17.7 apart), Z{BASE_T:g} to Z{CH_FLOOR:g}, one pass (inside the channel sketch, through the cleared gap)", tool=24,
       sketches=[ids["channel_sketch"]], side="inside", top=BASE_T, depth=BASE_T - CH_FLOOR, stepdown=BASE_T - CH_FLOOR, rpm=AL["fin_rpm"], feed=AL["fin_feed"]),
    op(3, "deburr", f"Chamfer {RAIL_CH} the rail tops outside (takes the 0.3 left on the band and hides the op 1 / op 2 seam)", tool=1,
       chamfer=RAIL_CH, tip_offset=1.0, edges=[[s * BW / 2, cy, BASE_T] for s in (1, -1) for cy in (-26.5, 26.5)]),
    op(3, "deburr", "Edge break 0.3: channel edges, floor ends, pocket rims", tool=1, chamfer=0.3, tip_offset=0.7,
       edges=[[s * CH_W / 2, cy, BASE_T] for s in (1, -1) for cy in (-26.5, 26.5)] + [[0, cy + s * 22.5, CH_FLOOR] for s in (1, -1) for cy in (-26.5, 26.5)]
       + [[4.025, cy + m, CH_FLOOR] for m in (-15, 0, 15) for cy in (-26.5, 26.5)]),
] + outs(3, BASED, "eclipse-base-thick-op2") + cycles(2, BASED, "eclipse-base-thick", ["Thick base op1 bottom"], ["Thick base op2 top"])
S += [{"tool": "save_document", "args": {"path": f"{HERE}/eclipse-car-cam.json"}}]
for st in S:
    if st["tool"] == "cam_op":
        st["args"]["build"] = False
json.dump(S, open(f"{HERE}/cam.json", "w"), indent=1)
k = next(i for i, st in enumerate(S) if st["tool"] == "create_cam" and st["args"]["name"] == "Thick base op1 bottom")
json.dump(S[:k] + [{"tool": "save_document", "args": {"path": f"{HERE}/eclipse-car-cam-caronly.json"}}], open(f"{HERE}/cam-car.json", "w"), indent=1)
print(len(S), "steps")
