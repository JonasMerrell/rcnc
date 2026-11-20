"""Machine models with the vise's moving jaws closed on the slider job's work, from ~/.config/cad-rust-app/machines/DMC2.json.

Left station (op 1): the moving insert bar's face goes to x -33.2 (25.4 bar against the centre block's face at -7.8).
Right station (op 2): to 25.5 + width (25.0 base, 19.6 slider). Bases move on the 25 mm grid, the top jaw and its
insert bar by the rest (the jaw screw). Mesh = zlib'd float32 x,y,z triangles; only x changes.
"""
import base64, json, os, struct, zlib

SRC = os.path.expanduser("~/.config/cad-rust-app/machines/DMC2.json")
DST = os.path.expanduser("~/.config/cad-rust-app/machines")


def shift(part, dx, dy=0.0):
    b = zlib.decompress(base64.b64decode(part["mesh"]))
    f = list(struct.unpack("<%df" % (len(b) // 4), b))
    for i in range(0, len(f), 3):
        f[i] += dx
        f[i + 1] += dy
    part["mesh"] = base64.b64encode(zlib.compress(struct.pack("<%df" % len(f), *f))).decode()


def box(name, lo, hi, color):
    """A machine part: a box on the table (carried by Y), from lo to hi in machine coordinates."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    v = [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0), (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)]
    faces = [(0, 2, 1), (0, 3, 2), (4, 5, 6), (4, 6, 7), (0, 1, 5), (0, 5, 4), (1, 2, 6), (1, 6, 5), (2, 3, 7), (2, 7, 6), (3, 0, 4), (3, 4, 7)]
    f = [c for t in faces for i in t for c in v[i]]
    return {"name": name, "carrier": "y", "color": color, "mesh": base64.b64encode(zlib.compress(struct.pack("<%df" % len(f), *f))).decode()}


def make(name, left_face=None, right_face=None, others=()):
    d = json.load(open(SRC))
    d["name"] = name
    by = {p["name"]: p for p in d["parts"]}
    moves = []
    if left_face is not None:   # left station: bar 3 face now at -47.8
        dx = left_face - (-47.8)
        holes = round(dx / 25)
        moves += [("Vise/M12 Adjustable Base", 25 * holes), ("Vise/Top Jaw", dx), ("Vise/Gen3 Smooth Insert Bar 3", dx)]
    if right_face is not None:  # right station: bar 4 face now at 143.3
        dx = right_face - 143.3
        holes = round(dx / 25)
        moves += [("Vise/M12 Adjustable Base 2", 25 * holes), ("Vise/Top Jaw 2", dx), ("Vise/Gen3 Smooth Insert Bar 4", dx)]
    # the CAD has the moving jaws 1.9 mm off the fixed ones along Y (bar 3 centre y -348.45, bar 4 -352.25,
    # fixed bars -350.35): line them up, as the real vise is
    dy = {"Vise/Top Jaw": -1.9, "Vise/Gen3 Smooth Insert Bar 3": -1.9, "Vise/Top Jaw 2": 1.9, "Vise/Gen3 Smooth Insert Bar 4": 1.9}
    for n, dx in moves:
        shift(by[n], dx, dy.get(n, 0.0))
        print(f"{name}: {n} moved {dx:+.1f} in X, {dy.get(n, 0.0):+.1f} in Y")
    for o in others:
        d["parts"].append(box(*o))
        print(f"{name}: + {o[0]}")
    json.dump(d, open(os.path.join(DST, name + ".json"), "w"))


# two-station cycle: both jaws closed, the other station's work standing in the vise at its highest (raw) as a
# box, so each station's setup checks the head, holder and tool against it. Table y -350.31, ledges z 34.435.
Y0, Y1, LEDGE = -350.31 - 50, -350.31 + 50, 34.435
GREY, GOLD = [170, 175, 182], [200, 160, 70]
for mat, h1, h2, w2, col in (("base", 12.7, 12.2, 25.0, GREY), ("slider", 10.5, 9.3, 19.6, GOLD)):
    left_blank = (f"left station: the {mat} blank for op 1 (raw)", (-33.2, Y0, LEDGE), (-7.8, Y1, LEDGE + h1), col)
    right_blank = (f"right station: the flipped {mat} blank for op 2 (before facing)", (25.5, Y0 + 1, LEDGE), (25.5 + w2, Y1 - 1, LEDGE + h2), col)
    make(f"DMC2-cycle-{mat}-L", left_face=-33.2, right_face=25.5 + w2, others=[right_blank])
    make(f"DMC2-cycle-{mat}-R", left_face=-33.2, right_face=25.5 + w2, others=[left_blank])
# for rCAD jobs (both stations in one setup's program): both jaws closed, rCAD places the other station's work
# the left station's moving jaw closes on the raw bar: its measured width (base 26.6, slider 25.0, 2026-11-20)
for mat, w1, w2 in (("base", 26.6, 25.0), ("slider", 25.0, 19.6)):
    make(f"DMC2-twostation-{mat}", left_face=-7.8 - w1, right_face=25.5 + w2)
make("DMC2-slider-op1", left_face=-7.8 - 25.4)
make("DMC2-slider-op2-base", right_face=25.5 + 25.0)
make("DMC2-slider-op2-slider", right_face=25.5 + 19.6)
