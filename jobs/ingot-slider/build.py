"""Ingot magnetic fidget slider: the rCAD document (CAD, assembly, CAM bodies). Writes build.json for `rcad run`.

Studios: 0 Base, 1 Slider, 2 Magnet (display, every edge chamfered as made), 3 Base x2 CAM, 4 Slider x2 CAM
(sharp edges: the chamfer and deburr operations put the edge breaks on, the 2.5D chamfer op needs the sharp
edge), assembly 0. X across the bar (the vise clamps along X), Y along it, Z up, part bottom at z 0.
"""
import json, math, os

# ---- sizes (mm) ----
BW, BL, BT = 25.0, 45.0, 8.6       # base width, length, thickness (6061): 8.6 for the measured 12.34 bar (Jonas, 2026-11-20)
CH_W, CH_D = 20.0, 3.0             # channel, full length: leaves 2.5 mm rails
BR = 2.5                           # base outline corner radius (= the rail width: the arc ends on the channel wall)
BCH = 1.7                          # base rail-top chamfer, outside: its lower edge (Z6.9) hides the op 1 / op 2 seam (op 1 walls reach Z7.14)
SW, SL, ST = 19.6, 45.0, 8.0       # slider (C360): 0.2 clearance each side in the channel
SR = 3.0                           # slider outline corner radius
SCH = 2.4                          # ingot chamfer, 45 deg: its lower edge (Z5.6) hides the op 1 / op 2 seam (op 1 walls reach Z5.8)
EB = 0.3                           # edge break everywhere else
MAG_D, MAG_H = 8.0, 3.0            # N52 disc
POCKET_D, POCKET_H = 8.05, 3.15    # magnet sits 0.15 below flush (glue)
SLIDER_POCKET_D = 8.07             # brass pockets 8.07 (Jonas, 2026-11-22: 8.05 came out tight after the first blank; the cutter is re-seated every tool change)
PITCH = 15.0
MAGS = [-PITCH, 0.0, PITCH]
PAIR = 26.5                        # two parts per blank: centres at y -26.5 / +26.5, 8 mm gap, 98 overall
FIELD = (6.4, 19.1)                # engraving field half sizes (top flat is 7.4 x 20.1 half: 1 mm in from the chamfer)
HATCH = 2.5                        # crosshatch line spacing (perpendicular), lines at +-45 deg


# closed rectangles beyond each outer end (y 49..62): a trace inside them runs along the end face, x +-16.8
END = PAIR + BL / 2
END_RECTS = [{"type": "rect", "corner1": [-20, END], "corner2": [20, END + 13]}, {"type": "rect", "corner1": [-20, -END - 13], "corner2": [20, -END]}]


def rrect(hx, hy, r, cx=0.0, cy=0.0):
    a, b = hx - r, hy - r
    P = lambda x, y: [round(cx + x, 4), round(cy + y, 4)]
    return [{"type": "line", "from": P(-a, -hy), "to": P(a, -hy)},
            {"type": "arc", "center": P(a, -b), "start": P(a, -hy), "end": P(hx, -b)},
            {"type": "line", "from": P(hx, -b), "to": P(hx, b)},
            {"type": "arc", "center": P(a, b), "start": P(hx, b), "end": P(a, hy)},
            {"type": "line", "from": P(a, hy), "to": P(-a, hy)},
            {"type": "arc", "center": P(-a, b), "start": P(-a, hy), "end": P(-hx, b)},
            {"type": "line", "from": P(-hx, b), "to": P(-hx, -b)},
            {"type": "arc", "center": P(-a, -b), "start": P(-hx, -b), "end": P(-a, -hy)}]


def outline_edges(hx, hy, r, z, cy=0.0, ends=True, sides=True):
    """Points on each edge of a rounded-rectangle outline at height z (edge refs)."""
    a, b = hx - r, hy - r
    c = r * math.sqrt(0.5)
    pts = []
    if sides:
        pts += [(hx, 0), (-hx, 0)]
    if ends:
        pts += [(0, hy), (0, -hy)]
    pts += [(a + c, b + c), (-a - c, b + c), (a + c, -b - c), (-a - c, -b - c)]
    return [{"kind": "edge", "point": [round(x, 4), round(cy + y, 4), z]} for x, y in pts]


class Doc:
    def __init__(self):
        self.S = [{"tool": "new_document", "args": {"units": "mm"}}]
        self.fid = 0   # feature ids count per studio from 1

    def studio(self, name, first=False):
        if first:
            self.S.append({"tool": "set_variable", "args": {"name": "pitch", "expr": str(PITCH)}})
        else:
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

    def extrude(self, sk, depth, op, name):
        return self.feat("extrude", {"profiles": [{"sketch": sk}], "depth": depth, "op": op}, name)

    def face(self, z, x=0.0, y=0.0):
        return {"kind": "face", "point": [x, y, z], "normal": [0, 0, 1]}

    def face_down(self, x=0.0, y=0.0):
        return {"kind": "face", "point": [x, y, 0], "normal": [0, 0, -1]}


def base(d, cy=0.0, chamfers=True):
    sk = d.sketch(rrect(BW / 2, BL / 2, BR, cy=cy))
    d.extrude(sk, BT, "add", "Base 25 x 45 x 9, R2.5 corners")
    sk = d.sketch([{"type": "rect", "corner1": [-CH_W / 2, cy - BL / 2 - 2], "corner2": [CH_W / 2, cy + BL / 2 + 2]}], d.face(BT, 11.25, cy))
    d.extrude(sk, CH_D, "remove", "Channel 20 x 3, full length")
    sk = d.sketch([{"type": "circle", "center": [0, cy + m], "radius": POCKET_D / 2} for m in MAGS], d.face(BT - CH_D, 0, cy + 7.5))
    d.extrude(sk, POCKET_H, "remove", "3 magnet pockets 8.05 x 3.15 at 15 pitch")
    if chamfers:
        hx, hy, r = BW / 2, BL / 2, BR
        d.feat("chamfer", {"edges": outline_edges(hx, hy, r, 0, cy), "distance": EB}, "Edge break 0.3 bottom outline (sits on the 1.9 ledges in op 2)")
        # top: the rails' outer edges and their corner arcs (the arcs end on the channel wall)
        top = outline_edges(hx, hy, r, BT, cy, ends=False)
        d.feat("chamfer", {"edges": top, "distance": BCH}, "Chamfer 1.7 rail tops, outside")
        inner = [{"kind": "edge", "point": [s * CH_W / 2, cy, BT]} for s in (1, -1)]
        floor_ends = [{"kind": "edge", "point": [0, cy + s * BL / 2, BT - CH_D]} for s in (1, -1)]
        d.feat("chamfer", {"edges": inner + floor_ends, "distance": EB}, "Edge break 0.3 channel edges")
        rims = [{"kind": "edge", "point": [POCKET_D / 2, cy + m, BT - CH_D]} for m in MAGS]
        d.feat("chamfer", {"edges": rims, "distance": EB}, "Edge break 0.3 pocket rims")


def slider(d, cy=0.0, chamfers=True):
    sk = d.sketch(rrect(SW / 2, SL / 2, SR, cy=cy))
    d.extrude(sk, ST, "add", "Slider 19.6 x 45 x 8, R3 corners")
    sk = d.sketch([{"type": "circle", "center": [0, cy + m], "radius": SLIDER_POCKET_D / 2} for m in MAGS], d.face_down(0, cy + 7.5))
    d.extrude(sk, POCKET_H, "remove", "3 magnet pockets 8.07 x 3.15 at 15 pitch (bottom)")
    if chamfers:
        d.feat("chamfer", {"edges": outline_edges(SW / 2, SL / 2, SR, ST, cy), "distance": SCH}, "Ingot chamfer 2.4 x 45")
        d.feat("chamfer", {"edges": outline_edges(SW / 2, SL / 2, SR, 0, cy), "distance": EB}, "Edge break 0.3 bottom outline")
        rims = [{"kind": "edge", "point": [SLIDER_POCKET_D / 2, cy + m, 0]} for m in MAGS]
        d.feat("chamfer", {"edges": rims, "distance": EB}, "Edge break 0.3 pocket rims")


def hatch(cy):
    """Default engraving: crosshatch at +-45 deg inside the field, plus a border line."""
    fx, fy = FIELD
    items = rrect(fx, fy, 1.0, cy=cy)
    step = HATCH * math.sqrt(2)
    for sgn in (1, -1):
        # lines x - sgn*y = c (in field coordinates)
        cmax = fx + fy
        k = math.floor(cmax / step)
        for i in range(-k, k + 1):
            c = i * step
            # clip x - sgn*y = c to the box
            pts = []
            for x in (-fx, fx):
                y = sgn * (x - c)
                if -fy - 1e-9 <= y <= fy + 1e-9:
                    pts.append((x, y))
            for y in (-fy, fy):
                x = c + sgn * y
                if -fx - 1e-9 <= x <= fx + 1e-9:
                    pts.append((x, y))
            pts = sorted(set((round(p[0], 4), round(p[1], 4)) for p in pts))
            if len(pts) >= 2 and math.dist(pts[0], pts[-1]) > 1.0:
                (x0, y0), (x1, y1) = pts[0], pts[-1]
                items.append({"type": "line", "from": [x0, round(cy + y0, 4)], "to": [x1, round(cy + y1, 4)]})
    return items


def eclipse_drawing(cy):
    """Second engraving option: a 1st-gen Eclipse in side view (nose toward +Y), traced from Jonas's blueprint
    (eclipse_drawing.json, pixels): outline, glass, B pillar, door, side crease, head and tail lamps, wheels. Car
    length along Y, height across X (roof toward +X). Lines and circles only (traced 0.3 deep with the chamfer mill tip)."""
    D = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "eclipse_drawing.json")))
    k = 37.6 / (D["x1"] - D["x0"])
    P = lambda x, y: [round((D["ground"] - y) * k - 5.6, 4), round(cy + 18.8 - (x - D["x0"]) * k, 4)]
    items = []
    for pts in D["lines"].values():
        items += [{"type": "line", "from": P(*a), "to": P(*b)} for a, b in zip(pts, pts[1:]) if a != b]
    for (x, y), r in D["circles"]:
        items.append({"type": "circle", "center": P(x, y), "radius": round(r * k, 4)})
    return items



d = Doc()
d.studio("Base", first=True)
base(d)
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Base (6061)", "color": "#c9ced6", "material": "Aluminium 6061", "finish": "metal"}})
d.studio("Slider")
slider(d)
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Slider (C360 brass)", "color": "#d9a640", "material": "Brass", "finish": "metal"}})
d.studio("Magnet")
sk = d.sketch([{"type": "circle", "center": [0, 0], "radius": MAG_D / 2}])
d.extrude(sk, MAG_H, "add", "N52 disc 8 x 3")
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Magnet N52 8 x 3", "color": "#6d727a", "finish": "glossy"}})
# CAM bodies: two of each, end to end, sharp edges
d.studio("Base x2 (CAM)")
for cy in (-PAIR, PAIR):
    base(d, cy, chamfers=False)
d.sketch(END_RECTS)   # 13: beyond the outer ends, for op 1's end trims (trace inside)
d.sketch([{"type": "rect", "corner1": [-10, -53.5], "corner2": [10, 53.5]}])   # 14: the channel, both bases and past the ends (op 2 wall trace inside)
d.studio("Slider x2 (CAM)")
for cy in (-PAIR, PAIR):
    slider(d, cy, chamfers=False)
for cy in (-PAIR, PAIR):
    d.sketch(hatch(cy), d.face(ST, 0, cy))      # 9, 10: the default engraving
d.sketch(END_RECTS)                              # 11
for cy in (-PAIR, PAIR):
    d.sketch(eclipse_drawing(cy), d.face(ST, 0, cy))   # 12, 13: the Eclipse side-view engraving (second option)
d.S.append({"tool": "describe", "args": {}})

# assembly: slider centred in the channel, magnets in their pockets
A = [{"tool": "create_assembly", "args": {"name": "Ingot slider"}},
     {"tool": "assembly_insert", "args": {"assembly": 0, "studio": 0, "name": "Base", "fixed": True}},
     {"tool": "assembly_insert", "args": {"assembly": 0, "studio": 1, "name": "Slider", "position": [0, 0, BT - CH_D]}}]
for m in MAGS:
    A.append({"tool": "assembly_insert", "args": {"assembly": 0, "studio": 2, "name": f"Base magnet y{m:+.0f}", "position": [0, m, BT - CH_D - POCKET_H]}})
    A.append({"tool": "assembly_insert", "args": {"assembly": 0, "studio": 2, "name": f"Slider magnet y{m:+.0f}", "position": [0, m, BT - CH_D + POCKET_H - MAG_H]}})
d.S += A
# exploded by hand: base magnets 8 up out of their pockets, slider 22 up, its magnets 8 below it
E = [{"tool": "create_assembly", "args": {"name": "Ingot slider, exploded"}},
     {"tool": "assembly_insert", "args": {"assembly": 1, "studio": 0, "name": "Base", "fixed": True}},
     {"tool": "assembly_insert", "args": {"assembly": 1, "studio": 1, "name": "Slider", "position": [0, 0, BT - CH_D + 22]}}]
for m in MAGS:
    E.append({"tool": "assembly_insert", "args": {"assembly": 1, "studio": 2, "name": f"Base magnet y{m:+.0f}", "position": [0, m, BT - CH_D - POCKET_H + 8]}})
    E.append({"tool": "assembly_insert", "args": {"assembly": 1, "studio": 2, "name": f"Slider magnet y{m:+.0f}", "position": [0, m, BT - CH_D + 22 - 8]}})
d.S += E
d.S.append({"tool": "save_document", "args": {"path": "/home/jonas/dmc2/slider/slider.json"}})
R = lambda studio, view, path, w=1400, h=1000: [{"tool": "switch_part_studio", "args": {"index": studio}}, {"tool": "render", "args": {"view": view, "path": path, "width": w, "height": h}}]
d.S += R(0, "iso", "cad-base-iso.png") + R(0, "top", "cad-base-top.png") + R(1, "iso", "cad-slider-iso.png") + R(1, "iso_front_left_below", "cad-slider-under.png") + R(4, "top", "cad-slider-cam-top.png") + R(3, "iso", "cad-base-cam.png")
d.S += [{"tool": "render_assembly", "args": {"assembly": 0, "view": "iso", "path": "asm-iso.png", "width": 1400, "height": 1000}},
        {"tool": "render_assembly", "args": {"assembly": 0, "view": "front", "path": "asm-front.png", "width": 1400, "height": 800}},
        {"tool": "render_assembly", "args": {"assembly": 1, "view": "iso", "path": "asm-exploded.png", "width": 1400, "height": 1200}}]
json.dump(d.S, open("/home/jonas/dmc2/slider/build.json", "w"), indent=1)
print(len(d.S), "steps")
