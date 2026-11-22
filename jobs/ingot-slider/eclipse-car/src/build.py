"""Eclipse car (approved surface, 2026-11-21; smooth-corner solid c0ad1f0 from 2026-11-22) and its thick-rail base: rCAD document eclipse-car.json.

The car is the approved surface itself: rCAD imports eclipse-car-top-0.15mm.stl (rcnc design/eclipse-car-surface
25ebef5) as a solid; clean.py only removes 785 pairs of opposite-facing triangles lying on the bottom (cells
outside the outline at z 0) so it is a closed manifold (same volume, 5250.425 mm3); mkstl.py turns it for the vise
(length along Y, nose at +Y). The magnet pockets are cut into it here (exact cylinders).

Studios: 0 Car (as designed), 1 Cars x2 (CAM: y -26.5 / +26.5, both up 1.67 = op 2's shims), 2 Thick-rail base
(as made, edge breaks), 3 Thick-rail bases x2 (CAM, sharp edges + sketches). Assembly 0: car on its base.
X across the bar (the vise clamps along X), Y along it, Z up, part bottom at z 0.
Run: python3 mkstl.py && python3 build.py && rcad run build.json"""
import json, math

HERE = "/home/jonas/dmc2/eclipse-car"
PAIR, SHIM_OP2 = 26.5, 1.67
POCKET_D, POCKET_H, MAGS = 8.05, 3.15, [-15.0, 0.0, 15.0]
# the thick-rail base: the ingot base (~/dmc2/slider/build.py) with the channel 17.7 wide instead of 20.0
BW, BL, BT = 25.0, 45.0, 8.6
CH_W, CH_D = 17.7, 3.0             # rails 3.65 wide (were 2.5)
BR, BCH, EB = 2.5, 1.7, 0.3
END = PAIR + BL / 2
END_RECTS = [{"type": "rect", "corner1": [-20, END], "corner2": [20, END + 13]}, {"type": "rect", "corner1": [-20, -END - 13], "corner2": [20, -END]}]


def rrect(hx, hy, r, cx=0.0, cy=0.0):
    a, b = hx - r, hy - r
    P = lambda x, y: [round(cx + x, 4), round(cy + y, 4)]
    return [{"type": "line", "from": P(-a, -hy), "to": P(a, -hy)}, {"type": "arc", "center": P(a, -b), "start": P(a, -hy), "end": P(hx, -b)},
            {"type": "line", "from": P(hx, -b), "to": P(hx, b)}, {"type": "arc", "center": P(a, b), "start": P(hx, b), "end": P(a, hy)},
            {"type": "line", "from": P(a, hy), "to": P(-a, hy)}, {"type": "arc", "center": P(-a, b), "start": P(-a, hy), "end": P(-hx, b)},
            {"type": "line", "from": P(-hx, b), "to": P(-hx, -b)}, {"type": "arc", "center": P(-a, -b), "start": P(-hx, -b), "end": P(-a, -hy)}]


def outline_edges(hx, hy, r, z, cy=0.0, ends=True, sides=True):
    a, b = hx - r, hy - r
    c = r * math.sqrt(0.5)
    pts = ([(hx, 0), (-hx, 0)] if sides else []) + ([(0, hy), (0, -hy)] if ends else [])
    pts += [(a + c, b + c), (-a - c, b + c), (a + c, -b - c), (-a - c, -b - c)]
    return [{"kind": "edge", "point": [round(x, 4), round(cy + y, 4), z]} for x, y in pts]


class Doc:
    def __init__(self):
        self.S = [{"tool": "new_document", "args": {"units": "mm"}}]
        self.fid = 0

    def studio(self, name, first=False):
        if not first:
            self.S.append({"tool": "new_part_studio", "args": {"name": name}})
        self.fid = 0

    def sketch(self, items, plane=None):
        self.fid += 1
        self.S.append({"tool": "create_sketch", "args": {"plane": plane or {"kind": "origin_plane", "plane": "Top"}}})
        self.S.append({"tool": "sketch_add", "args": {"sketch_id": self.fid, "items": items}})
        return self.fid

    def feat(self, kind, params, name):
        self.fid += 1
        self.S.append({"tool": "add_feature", "args": {"kind": kind, "params": params, "name": name}})
        return self.fid

    def imp(self, path, name):
        self.fid += 1
        self.S.append({"tool": "import_file", "args": {"path": path, "name": name}})
        return self.fid

    def plane_z(self, z, name):
        return {"kind": "feature_plane", "feature": self.feat("plane", {"mode": "offset", "base": {"kind": "origin_plane", "plane": "Top"}, "offset": str(z)}, name)}

    def face(self, z, x=0.0, y=0.0):
        return {"kind": "face", "point": [x, y, z], "normal": [0, 0, 1]}


def pockets(d, cys, z0):
    pl = d.plane_z(z0, "car bottom") if z0 else {"kind": "origin_plane", "plane": "Top"}
    sk = d.sketch([{"type": "circle", "center": [0, cy + m], "radius": POCKET_D / 2} for cy in cys for m in MAGS], pl)
    d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": POCKET_H, "op": "remove"}, "3 magnet pockets 8.05 x 3.15 at x -15/0/+15 (bottom)")


def base(d, cy=0.0, chamfers=True):
    sk = d.sketch(rrect(BW / 2, BL / 2, BR, cy=cy))
    d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": BT, "op": "add"}, "Base 25 x 45 x 8.6, R2.5 corners")
    sk = d.sketch([{"type": "rect", "corner1": [-CH_W / 2, cy - BL / 2 - 2], "corner2": [CH_W / 2, cy + BL / 2 + 2]}], d.face(BT, 11.25, cy))
    d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": CH_D, "op": "remove"}, "Channel 17.7 x 3, full length (rails 3.65)")
    sk = d.sketch([{"type": "circle", "center": [0, cy + m], "radius": POCKET_D / 2} for m in MAGS], d.face(BT - CH_D, 0, cy + 7.5))
    d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": POCKET_H, "op": "remove"}, "3 magnet pockets 8.05 x 3.15 at 15 pitch")
    if chamfers:
        d.feat("chamfer", {"edges": outline_edges(BW / 2, BL / 2, BR, 0, cy), "distance": EB}, "Edge break 0.3 bottom outline")
        d.feat("chamfer", {"edges": outline_edges(BW / 2, BL / 2, BR, BT, cy, ends=False), "distance": BCH}, "Chamfer 1.7 rail tops, outside")
        inner = [{"kind": "edge", "point": [s * CH_W / 2, cy, BT]} for s in (1, -1)]
        floor_ends = [{"kind": "edge", "point": [0, cy + s * BL / 2, BT - CH_D]} for s in (1, -1)]
        d.feat("chamfer", {"edges": inner + floor_ends, "distance": EB}, "Edge break 0.3 channel edges")
        d.feat("chamfer", {"edges": [{"kind": "edge", "point": [POCKET_D / 2, cy + m, BT - CH_D]} for m in MAGS], "distance": EB}, "Edge break 0.3 pocket rims")


d = Doc()
CAR_W, CAR_L, NOSE_R, TAIL_R = 17.3, 45.0, 4.8, 2.4


def car_outline(cy=0.0):
    """The plan outline exactly: 45 x 17.3, R4.8 at the nose (+Y), R2.4 at the tail (-Y). The STL's own outline is
    its 0.15 mm grid at the corners: stair-stepped, up to 0.27 outside these arcs (never inside)."""
    hx, hy, a, t = CAR_W / 2, CAR_L / 2, CAR_W / 2 - NOSE_R, CAR_W / 2 - TAIL_R
    P = lambda x, y: [round(x, 4), round(cy + y, 4)]
    return [{"type": "line", "from": P(-t, -hy), "to": P(t, -hy)},
            {"type": "arc", "center": P(t, -hy + TAIL_R), "start": P(t, -hy), "end": P(hx, -hy + TAIL_R)},
            {"type": "line", "from": P(hx, -hy + TAIL_R), "to": P(hx, hy - NOSE_R)},
            {"type": "arc", "center": P(a, hy - NOSE_R), "start": P(hx, hy - NOSE_R), "end": P(a, hy)},
            {"type": "line", "from": P(a, hy), "to": P(-a, hy)},
            {"type": "arc", "center": P(-a, hy - NOSE_R), "start": P(-a, hy), "end": P(-hx, hy - NOSE_R)},
            {"type": "line", "from": P(-hx, hy - NOSE_R), "to": P(-hx, -hy + TAIL_R)},
            {"type": "arc", "center": P(-t, -hy + TAIL_R), "start": P(-hx, -hy + TAIL_R), "end": P(-t, -hy)}]


def clip_outline(d, cys, z0=0.0):
    sk = d.sketch([it for cy in cys for it in car_outline(cy)])
    d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": 15, "op": "intersect"}, "Plan outline exactly R4.8 nose / R2.4 tail (trims the STL's stair-stepped corners)")
    # the STL's corner cells dip below 3.9 (its grid's partial cells, down to z 0.2): fill them, straight walls to 3.9
    pl = d.plane_z(z0, "car bottom (outline)") if z0 else {"kind": "origin_plane", "plane": "Top"}
    sk = d.sketch([it for cy in cys for it in car_outline(cy)], pl)
    d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": 3.9, "op": "add"}, "Straight walls to z 3.9 all round (fills the STL's corner dips: nothing shaped below 3.9)")


d.studio("Eclipse car", first=True)
d.imp(f"{HERE}/car.stl", "Approved car solid (eclipse-car-solid-smooth.stl: exact R4.8 / R2.4 walls, nose +Y)")
pockets(d, [0.0], 0.0)
d.S.append({"tool": "set_part", "args": {"body": [1, 0], "name": "Eclipse car (C360 brass)", "color": "#d9a640", "material": "Brass", "finish": "metal"}})
d.studio("Eclipse cars x2 (CAM)")
d.imp(f"{HERE}/cars-cam.stl", "Two approved cars at y -26.5 / +26.5, up 1.67 (op 2's shims)")
pockets(d, [-PAIR, PAIR], SHIM_OP2)
# the straight part of the nose top edge (z 3.9), for op 2's T1 edge break as a trace (rCAD's deburr ran on
# round the smooth R4.8 corners onto the rising fenders and gouged them)
ends_car = d.sketch([{"type": "line", "from": [-3.6, cy + 22.5], "to": [3.6, cy + 22.5]} for cy in (-PAIR, PAIR)])
d.studio("Thick-rail base")
base(d)
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Thick-rail base (6061)", "color": "#c9ced6", "material": "Aluminium 6061", "finish": "metal"}})
d.studio("Thick-rail bases x2 (CAM)")
for cy in (-PAIR, PAIR):
    base(d, cy, chamfers=False)
ends = d.sketch(END_RECTS)
chan = d.sketch([{"type": "rect", "corner1": [-CH_W / 2, -53.5], "corner2": [CH_W / 2, 53.5]}])
d.S.append({"tool": "describe", "args": {}})
R = lambda st, v, p, w=1400, h=1000: [{"tool": "switch_part_studio", "args": {"index": st}}, {"tool": "render", "args": {"view": v, "path": f"{HERE}/{p}", "width": w, "height": h}}]
for v in ("iso", "right", "top", "front", "iso_front_left_below"):
    d.S += R(0, v, f"car-{v}.png")
d.S += R(1, "iso", "cars-cam-iso.png") + R(2, "iso", "base-iso.png") + R(2, "front", "base-front.png") + R(3, "iso", "bases-cam-iso.png")
d.S += [{"tool": "create_assembly", "args": {"name": "Eclipse car on the thick-rail base"}},
        {"tool": "assembly_insert", "args": {"assembly": 0, "studio": 2, "name": "Base", "fixed": True}},
        {"tool": "assembly_insert", "args": {"assembly": 0, "studio": 0, "name": "Car", "position": [0, 0, BT - CH_D]}},
        {"tool": "render_assembly", "args": {"assembly": 0, "view": "iso", "path": f"{HERE}/asm-iso.png", "width": 1400, "height": 1000}},
        {"tool": "render_assembly", "args": {"assembly": 0, "view": "front", "path": f"{HERE}/asm-front.png", "width": 1400, "height": 800}},
        {"tool": "render_assembly", "args": {"assembly": 0, "view": "right", "path": f"{HERE}/asm-right.png", "width": 1400, "height": 800}}]
d.S.append({"tool": "save_document", "args": {"path": f"{HERE}/eclipse-car.json"}})
json.dump(d.S, open(f"{HERE}/build.json", "w"), indent=1)
json.dump({"ends_sketch": ends, "channel_sketch": chan, "car_end_edges": ends_car}, open(f"{HERE}/sketch_ids.json", "w"))
print(len(d.S), "steps; base sketches", ends, chan)
