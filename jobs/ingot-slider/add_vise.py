"""Find stock from the saved vise location (rcnc PR 21/22, FIND_STOCK VISE=): no jogging to the stock first.

Op 1 (G54, left station): the bar lies against the centre block on its +X side -> VISE=1.
Op 2 (G55, right station): the finished op 1 wall lies against the centre block on its -X side -> VISE=-1, and
JAWAT= the work X of that wall (the raw band above it, where the sides are touched, is wider).
Run after posting; leaves programs that already have VISE= alone.

    python3 add_vise.py base-first.nc ...
"""
import re
import sys

WALL = {"base": 25.0, "slider": 19.6, "eclipse": 19.6, "engraved": 19.6}   # op 1's finished width, the face op 2 grips
for p in sys.argv[1:]:
    part = p.replace("\\", "/").split("/")[-1].split("-")[0]
    out, station, n = [], None, 0
    for line in open(p).read().split("\n"):
        s = line.strip()
        if s in ("G54", "G55"):
            station = s
        if s.startswith("(RCNC FIND_STOCK") and "VISE=" not in s:
            assert station, f"{p}: Find stock before a work offset"
            add = " VISE=1" if station == "G54" else f" VISE=-1 JAWAT={-WALL[part] / 2:g}"
            line = line.rstrip()[:-1] + add + ")"
            n += 1
        out.append(line)
    open(p, "w").write("\n".join(out))
    print(p, "vise:", n)
