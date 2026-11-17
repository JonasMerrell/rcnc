"""Stick-out setting gauge: the rCAD document and the concept renders. Writes build.json for `rcad run`.

A 20 x 88 x 16 mm 6061 bar with open-ended slots running in from its machined end faces. Slot length =
stick-out: push the gauge's end face up against the ER16 nut face, the tool slides down the slot until its
tip rests on the slot's end wall, tighten. Top face: 21, 22, 24, 26 and a thumb dish; bottom face: 18, 19,
20, 23. Numbers engraved beside each slot.

Studios: 0 Gauge (finished, edges broken), 1 Gauge CAM (sharp edges), 2 Op 1 blank (the 25.4 bar with the end
lugs and their bores, as op 1 leaves it), 3 In use (gauge + a 1/4" tool at 24 + an ER16 nut, for the picture).
X across the bar (the vise clamps along X), Y along it, Z up; part bottom at z 0, centred.
"""
import json, math

W, L, H = 20.0, 88.0, 16.0         # gauge
SW, SD = 7.0, 6.0                  # slot width (6.35 / 6 tools slide in), depth
SX = 5.0                           # slot centres at x +-5: 1.5 outer walls, 3 web
# (face, end, x, stick-out): end +1 = +Y
SLOTS = [("top", 1, 5, 24), ("top", 1, -5, 22), ("top", -1, 5, 21), ("top", -1, -5, 26),
         ("bottom", 1, 5, 19), ("bottom", 1, -5, 18), ("bottom", -1, 5, 20), ("bottom", -1, -5, 23)]
DISH_D, DISH_H = 16.0, 3.5         # thumb dish, spherical, top face middle
CH, EB = 0.5, 0.3                  # outer edge chamfer, edge break elsewhere
TXT_H, TXT_D = 4.0, 0.3            # engraved numbers
BAR, BAR_L = 25.4, 120.0           # stock: 1" square bar, bandsawn to 120 (+-1)
CARRIER = 3.0                      # stock below the gauge in op 1 (removed in op 2)
LUG_T, LUG_Y, LUG_BORE = 8.0, 52.0, 8.0   # end lugs: 8 thick in op 1, bore centres at y +-52, Ø8 through


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

    def extrude(self, sk, depth, op, name, **kw):
        return self.feat("extrude", {"profiles": [{"sketch": sk}], "depth": depth, "op": op, **kw}, name)


def face(z, x=0.0, y=0.0, up=True):
    return {"kind": "face", "point": [x, y, z], "normal": [0, 0, 1 if up else -1]}


def rect(x0, y0, x1, y1):
    return {"type": "rect", "corner1": [x0, y0], "corner2": [x1, y1]}


def slot_rect(end, x, s, over=2.0):
    """The slot's plan: from `over` past the end face to s in from it."""
    e = end * L / 2
    return rect(x - SW / 2, e - end * s, x + SW / 2, e + end * over)


def gauge(d, z0=0.0, sharp=False):
    sk = d.sketch([rect(-W / 2, -L / 2, W / 2, L / 2)])
    d.extrude(sk, H, "add", "Gauge 20 x 88 x 16") if z0 == 0 else None
    for fc, up, z in (("top", True, H), ("bottom", False, 0.0)):
        items = [slot_rect(e, x, s) for f, e, x, s in SLOTS if f == fc]
        sk = d.sketch(items, face(z, 0, 0, up))
        d.extrude(sk, SD, "remove", f"{fc.capitalize()} slots 7 wide x 6 deep: " + ", ".join(str(s) for f, e, x, s in SLOTS if f == fc))
    # thumb dish: a spherical cap, revolved about Z
    r = DISH_D / 2
    R = (r * r + DISH_H * DISH_H) / (2 * DISH_H)
    zc = H - DISH_H + R
    a1 = math.atan2(H - zc, r)
    sk = d.sketch([{"type": "arc", "center": [0, zc], "start": [0, H - DISH_H], "end": [r, H]} if False else
                   {"type": "polyline", "points": [[0, H - DISH_H]] + [[round(R * math.cos(-math.pi / 2 + (math.pi / 2 + a1) * k / 24), 4), round(zc + R * math.sin(-math.pi / 2 + (math.pi / 2 + a1) * k / 24), 4)] for k in range(1, 25)] + [[0, H + 0.5]], "closed": True}],
                  {"kind": "origin_plane", "plane": "Front"})
    d.feat("revolve", {"profiles": [{"sketch": sk}], "axis": {"kind": "axis", "axis": 2}, "op": "remove"}, "Thumb dish Ø16 x 3.5")
    # numbers beside each slot's inner end, 0.3 deep
    for fc, up, z in (("top", True, H), ("bottom", False, 0.0)):
        items = []
        for f, e, x, s in SLOTS:
            if f != fc:
                continue
            y = e * (L / 2 - s - 3.2)
            # read from the open end: on the bottom face the text is mirrored in X so it reads right side up there
            tx = x if up else -x
            items.append({"type": "text", "at": [tx - 3.2, y - TXT_H / 2], "text": str(s), "height": TXT_H})
        # left as sketches: the CAM engraves them (a text extrude leaves stray mesh points that spoil renders)
        d.sketch(items, face(z, 0, 14, up))


d = Doc()
d.studio("Gauge", first=True)
gauge(d)
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Stick-out gauge (6061)", "color": "#c9ced6", "material": "Aluminium 6061", "finish": "metal"}})
# 1: after op 1, in op 1's frame (bar bottom on the parallels at z 0): the 25.4 bar faced to z 19 (the calibration
# ladder and the facing took the top 6.4), the gauge's top features and walls down to z 5 (above the jaws),
# 14 mm end lugs left 8 thick with the Ø8 through bores op 2 finds itself by, the 3 mm carrier band under it
d.studio("After op 1 (op 1 frame)")
Z1 = CARRIER                       # gauge bottom in op 1
sk = d.sketch([rect(-BAR / 2, -BAR_L / 2, BAR / 2, BAR_L / 2)])
d.extrude(sk, CARRIER + H, "add", "Bar faced to z 19 (25.4 wide, 120 long)")
sk = d.sketch([rect(-BAR / 2 - 1, -L / 2, -W / 2, L / 2), rect(W / 2, -L / 2, BAR / 2 + 1, L / 2)], face(CARRIER + H, 0, 0))
d.extrude(sk, CARRIER + H - 5.0, "remove", "Gauge walls down to z 5 (the jaws grip 4.7)")
sk = d.sketch([rect(-BAR / 2 - 1, L / 2, BAR / 2 + 1, BAR_L / 2 + 1), rect(-BAR / 2 - 1, -BAR_L / 2 - 1, BAR / 2 + 1, -L / 2)], face(CARRIER + H, 0, 0))
d.extrude(sk, CARRIER + H - LUG_T, "remove", "End lugs down to 8 thick")
sk = d.sketch([{"type": "circle", "center": [0, s * LUG_Y], "radius": LUG_BORE / 2} for s in (1, -1)], face(LUG_T, 0, LUG_Y))
d.extrude(sk, LUG_T, "remove", "Ø8 through bores in the lugs (op 2 locates from them)")
items = [slot_rect(e, x, s_) for f, e, x, s_ in SLOTS if f == "top"]
sk = d.sketch(items, face(CARRIER + H, 0, 14))
d.extrude(sk, SD, "remove", "Top slots")
r_ = DISH_D / 2
R_ = (r_ * r_ + DISH_H * DISH_H) / (2 * DISH_H)
top = CARRIER + H
zc = top - DISH_H + R_
a1 = math.atan2(top - zc, r_)
sk = d.sketch([{"type": "polyline", "points": [[0, top - DISH_H]] + [[round(R_ * math.cos(-math.pi / 2 + (math.pi / 2 + a1) * k / 24), 4), round(zc + R_ * math.sin(-math.pi / 2 + (math.pi / 2 + a1) * k / 24), 4)] for k in range(1, 25)] + [[0, top + 0.5]], "closed": True}], {"kind": "origin_plane", "plane": "Front"})
d.feat("revolve", {"profiles": [{"sketch": sk}], "axis": {"kind": "axis", "axis": 2}, "op": "remove"}, "Thumb dish")
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Bar after op 1", "color": "#b8c0cc", "material": "Aluminium 6061", "finish": "metal"}})
# 2: in use: the gauge with a 1/4" tool set to 24 and an ER16 nut against the end face (lying down for the picture)
d.studio("In use")
gauge(d)
d.S.append({"tool": "set_part", "args": {"body": [2, 0], "name": "Gauge", "color": "#c9ced6", "finish": "metal"}})
z_ax = H - SD + 6.35 / 2
sk = d.sketch([{"type": "circle", "center": [5, z_ax], "radius": 6.35 / 2}], {"kind": "origin_plane", "plane": "Front"})
tool = d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": 21 + 40, "op": "new", "end": "blind"}, "1/4 tool")
d.feat("transform", {"bodies": [{"kind": "body", "body": [tool, 0]}], "mode": "translate", "direction": {"kind": "axis", "axis": 1}, "distance": "-23"}, "Tool tip on the 21 slot's end wall (21 out of the nut)")
sk = d.sketch([{"type": "polygon", "center": [5, z_ax], "radius": 14, "sides": 8}], {"kind": "origin_plane", "plane": "Front"})
nut = d.feat("extrude", {"profiles": [{"sketch": sk}], "depth": 17.5, "op": "new"}, "ER16 nut (28 across)")
d.feat("transform", {"bodies": [{"kind": "body", "body": [nut, 0]}], "mode": "translate", "direction": {"kind": "axis", "axis": 1}, "distance": "-44"}, "Nut face on the gauge's end face")
d.S.append({"tool": "describe", "args": {}})
d.S.append({"tool": "save_document", "args": {"path": "/home/jonas/dmc2/gauge/gauge.json"}})
R = lambda view, path, w=1400, h=1000: {"tool": "render", "args": {"view": view, "path": path, "width": w, "height": h, "sketches": "unused"}}
SW_ = lambda i: {"tool": "switch_part_studio", "args": {"index": i}}
G = "/home/jonas/dmc2/gauge/"
d.S += [SW_(0), R("iso", G + "cad-iso.png"), R("top", G + "cad-top.png"), R("iso_front_left_below", G + "cad-under.png"),
        SW_(1), R("iso", G + "after-op1-iso.png"), R("front", G + "after-op1-front.png"),
        SW_(2), R("iso", G + "in-use.png")]
json.dump(d.S, open("/home/jonas/dmc2/gauge/build.json", "w"), indent=1)
print(len(d.S), "steps")
