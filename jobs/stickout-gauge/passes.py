"""The straight passes op 1 (top) and op 2 (bottom) take their bulk off with, re-planned for the 32 x 32 bar and
the spindle pull-out model after the first run broke T24. Jonas's rules: step over <= 1 mm (15% D), bulk <= ~450 W
by the model, peak < 500 W, straight passes for the band, the lugs and the side strips.

Each pass is a tool-centre line in op 1's part frame (X across, Y along, the bar's bottom at z 0), cut climb:
the material on the tool's left as it runs. Op 2's setup is op 1's turned about X (y -> -y, z -> -z), so its
lines are given in the same part frame and its seams are flipped to keep them climbing.

Where the bar is after op 1a v2 (op 1 frame): z 32 from x -9.7 to +6.4; +X side cut to z 26 in to x +6.4;
-X side cut to z 20 in to x -9.7.
"""
import contextlib
import io
with contextlib.redirect_stdout(io.StringIO()):
    import plan
predict = plan.predict

R = 6.35 / 2
BAR, BAR_L, L, W = 32.0, 120.7, 88.0, 23.0
F = 2400


def strips(x_from, x_to, w_max):
    """Equal strips no wider than w_max from x_from toward x_to: [(low edge, high edge)] in cutting order."""
    n = max(1, int(-(-abs(x_to - x_from) // w_max)) if abs(x_to - x_from) / w_max % 1 > 1e-9 else round(abs(x_to - x_from) / w_max))
    w = abs(x_to - x_from) / n
    s = 1 if x_to > x_from else -1
    out = []
    for k in range(n):
        a, b = x_from + s * k * w, x_from + s * (k + 1) * w
        out.append((min(a, b), max(a, b)))
    return out, w


def y_line(lo, hi, mat_neg_x):
    """Tool centre x for a Y pass taking strip lo..hi with the material on its -X side (or +X)."""
    return round(lo + R if mat_neg_x else hi - R, 4)


# ---- op 1 (top up) ----
OP1 = []   # (group, x, top, depth, w, seam_y, label)
# band level 1: the middle z 32 -> 26, from x +6.4 in to -9.7 (material on -X, run +Y)
st, w = strips(6.4, -9.7, 0.85)
for lo, hi in st:
    OP1.append(("band", y_line(lo, hi, True), 32.0, 6.0, w, -1000.0, f"Band z32-26, strip x {lo:+.2f}..{hi:+.2f}"))
# band level 2: z 26 -> 19.2, from x +16 in to -9.7, then the -X side's last 0.8 (z 20 -> 19.2) to x -16
st, w2 = strips(16.0, -9.7, 0.75)
for lo, hi in st:
    OP1.append(("band", y_line(lo, hi, True), 26.0, 6.8, w2, -1000.0, f"Band z26-19.2, strip x {lo:+.2f}..{hi:+.2f}"))
st, w3 = strips(-9.7, -16.0, 1.0)
for lo, hi in st:
    OP1.append(("band", y_line(lo, hi, True), 20.0, 0.8, w3, -1000.0, f"Band z20-19.2, strip x {lo:+.2f}..{hi:+.2f}"))

# lugs (|y| > 44): z 19 -> 8, lines across X from each free end in, two 5.5 levels each; the last 0.3 off the
# gauge's end face (the wall finish takes it). +Y lug runs -X, -Y lug runs +X (material toward the gauge: left)
LUG_W = 0.9
LUG = []   # (end, y)
for e in (1, -1):
    first = BAR_L / 2 + R - LUG_W
    last = L / 2 + 0.3 + R
    n = int(-(-(first - last) // LUG_W))
    step = (first - last) / n
    for k in range(n + 1):
        LUG.append((e, round(e * (first - k * step), 4)))
lug_w = max(LUG_W, (BAR_L / 2 + R - (LUG[0][1])))   # the first strip's width
# side strips beside the gauge (|x| 11.8 .. 16): z 19 -> 8.5 in two 5.25 levels, 5 strips a side from the bar's edge in
SIDE = []   # (side, x, w)
for sgn in (1, -1):
    st, ws = strips(16.0, 11.8, 1.0)
    for lo, hi in st:
        if sgn > 0:
            SIDE.append((1, y_line(lo, hi, True), ws))
        else:
            SIDE.append((-1, y_line(-hi, -lo, False), ws))

# ---- op 2 (bottom up; op 2 frame z = 19 - op 1 z) ----
# the carrier z 19 -> 16.2 (op 2 frame), 1.0 strips across the whole bar. In op 2's machine frame each runs +Y with
# the material on -X: in the part frame that is -Y, from part +X in.
st, wc = strips(16.0, -16.0, 1.0)
CARRIER = [(y_line(lo, hi, True), wc) for lo, hi in st]
# op 2's side strips (op 2 z 16 -> 14: what op 1 left beside the walls over the carrier), 5 a side
SIDE2 = SIDE

if __name__ == "__main__":
    def show(name, w, d):
        m = predict(w, d, F, 24000)
        print(f"{name:34s} {w:.3f} x {d:<4g} F{F}: {m['p']:4.0f} W  MRR {m['mrr']:4.1f} cm3/min")
    print(len(OP1), "band passes;", len(LUG), "lug lines;", len(SIDE), "side lines;", len(CARRIER), "carrier lines")
    show("band 1 (z32-26)", w, 6.0)
    show("band 2 (z26-19.2)", w2, 6.8)
    show("band -X (z20-19.2)", w3, 0.8)
    show("lug strip, 5.5 level", LUG_W, 5.5)
    show("lug first strip", lug_w, 5.5)
    show("side strip, 5.25 level", SIDE[0][2], 5.25)
    show("op 2 carrier", wc, 2.8)
    show("op 2 lug (5 deep real)", LUG_W, 5.5)
    print([y for e, y in LUG])

# ---- op 1 re-planned for S9000 (2026-11-18, the VFD at F004 = 150): the bar after op1a v2-v6 (and maybe v7) ----
# above Z20 the bar's -X side is gone to x +0.25; between Z20 and Z26 solid from x +0.25 to +16; above Z26 a rib
# x 0.25..6.4 to Z32.09. Band to Z19.2 in three sets, then the face finish; worst case without v7 (more stock).
OP1_9K = []   # (x, top, depth, w, seam_y, label)
# set 1: x 0.25 -> 6.4, the rib and the layer under it, 12.9 deep, material on the +X side (runs -Y)
st, w1 = strips(0.25, 6.4, 0.9)
for lo, hi in st:
    OP1_9K.append((y_line(lo, hi, False), 32.1, 12.9, w1, 1000.0, f"Band x {lo:+.2f}..{hi:+.2f}, z32.1-19.2"))
# set 2: x 6.4 -> 16, z26 -> 19.2, material on the +X side
st, w2b = strips(6.4, 16.0, 1.0)
for lo, hi in st:
    OP1_9K.append((y_line(lo, hi, False), 26.0, 6.8, w2b, 1000.0, f"Band x {lo:+.2f}..{hi:+.2f}, z26-19.2"))
# set 3: the -X side's last 0.8 (z20 -> 19.2), from x 0.25 out to -16, material on the -X side (runs +Y)
st, w3b = strips(0.25, -16.0, 1.0)
for lo, hi in st:
    OP1_9K.append((y_line(lo, hi, True), 20.0, 0.8, w3b, -1000.0, f"Band x {lo:+.2f}..{hi:+.2f}, z20-19.2"))
