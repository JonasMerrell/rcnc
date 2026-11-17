"""The calibration passes that op 1 takes its waste off with: their order and place, and what the cutting model
(rCAD cam_cutting: Kienzle kc1 700 / mc 0.25 for 6061, the DMC2's 1.5 kW x 0.78 spindle, constant torque below
24 000 rpm) predicts for each. Writes calibration.md (the table) and calibration-passes.png (where they cut).

Op 1's frame: the bar's bottom on the parallels at z 0, X across (+-16), Y along (the bar is 120 long; passes
run y -63 -> +63, in from air at one end and out at the other). T24 (1/4" 3 flute carbide, stick-out 24).
Every pass is a straight climb cut along +Y with the material on the tool's -X side.
"""
import math

D, Z_TEETH = 6.35, 3
KC1, MC = 700.0, 0.25
P_IN, EFF, RATED = 1500.0, 0.78, 24000.0
LEN = 134.0   # y -67 -> +67: the tool's radius + 3.5 past each sawn end (60.35)
TOP = 32.0
BAR = 32.0   # the bar is 32 x 32 (measured), not 1"


def predict(w, d, f, rpm):
    fz = f / (rpm * Z_TEETH)
    phi = math.acos(max(-1.0, min(1.0, 1 - 2 * min(w, D) / D)))
    hm = fz * (1 - math.cos(phi)) / phi
    kc = KC1 * max(hm, 0.005) ** -MC
    p = f * kc * w * d / 60000.0                       # W at the tool
    avail = P_IN * EFF * min(rpm / RATED, 1.0)
    v = math.pi * D * rpm / 60000.0                    # m/s
    force = p / v                                      # mean tangential force, N
    return dict(fz=fz, hm=hm, kc=kc, p=p, pct=100 * p / avail, force=force, mrr=w * d * f / 1000)


passes = []   # (id, what, w, d, f, rpm, x_low_edge, z_floor, side): the strip x_low_edge .. +w; side +1: cut from +X (climb +Y)
x = BAR / 2
# no air passes: running the program once with rcnc's Air cut on (+25) gives every pass's baseline at its own
# speed and feed (logged to <program>.air.cutlog.json)
n = 0


def cut(what, w, d, f, rpm):
    """A strip in from the +X side of the top band: material on the tool's -X side, climb along +Y."""
    global x, n
    x -= w
    n += 1
    passes.append((f"C{n}", what, w, d, f, rpm, x, TOP - d, 1))


xn = -BAR / 2


def cut_neg(what, w, d, f, rpm):
    """A strip in from the -X side: material on the tool's +X side, climb along -Y."""
    global xn, n
    n += 1
    passes.append((f"C{n}", what, w, d, f, rpm, xn, TOP - d, -1))
    xn += w


# top band from +X, 6 deep (z 32 -> 26): width ladder, then feed, then the rpm sweep at one fixed cut
for w in (0.6, 1.0, 1.4, 1.8):
    cut("width ladder", w, 6.0, 2400, 24000)
for f in (1800, 3000):
    cut("feed ladder", 1.4, 6.0, f, 24000)
for rpm in (18000, 19500, 21000, 22500, 24000):
    cut("rpm sweep (same chip)", 1.0, 6.0, rpm // 10, rpm)
cut("repeat of C3 (drift, repeatability)", 1.4, 6.0, 2400, 24000)
# shallower last: a deeper pass after them would also cut what they left under them
for d in (4.5, 3.0):
    cut("depth ladder", 1.4, d, 2400, 24000)
band_wall = x
# from -X, 12 deep (z 32 -> 20, above the gauge's top at 19): 0.3, Jonas's known 0.6 x 12 at 2500, then 0.9
# slower to stay under 75%
for w, f, what in ((0.3, 2400, "12 deep"), (0.6, 2500, "12 deep: Jonas's known 0.6 x 12 at 2500"), (0.9, 2000, "12 deep, wider (F2000 keeps it under 75%)")):
    cut_neg(what, w, 12.0, f, 24000)
# then op 1 takes the rest of the band (x xn .. band_wall, and what the depth ladder left) and the side strips

rows = []
total = 0.0
for pid, what, w, d, f, rpm, xw, zf, side in passes:
    t = LEN / f * 60
    total += t
    if w == 0:
        rows.append(f"| {pid} | {what} | – | – | {f} | {rpm} | 0 (no load) | – | – | {t:.1f} | above the stock, z {zf:.1f} |")
        continue
    m = predict(w, d, f, rpm)
    rows.append(
        f"| {pid} | {what} | {w:g} | {d:g} | {f} | {rpm} | **{m['pct']:.0f}%** ({m['p']:.0f} W) | {m['force']:.0f} | {m['hm'] * 1000:.1f} µm | {t:.1f} | x {xw:+.2f} .. {xw + w:+.2f}, z {zf:.1f} |")

head = """| Pass | What | Width ae (mm) | Depth ap (mm) | Feed (mm/min) | rpm | Predicted spindle | Mean cutting force (N) | Mean chip | Steady cut (s) | Where (op 1 frame) |
|---|---|---|---|---|---|---|---|---|---|---|"""
with open("calibration.md", "w") as fo:
    fo.write("# Calibration passes (op 1's waste)\n\n")
    fo.write("T24 (1/4\" 3-flute DLC carbide, stick-out 24), climb, straight along +Y over the whole 120 bar (126 mm "
             "with the run-in from air), one pass at a time with a rapid back over the top. Chatter margin and tool "
             "deflection per pass come from rCAD's simulation once the passes are rCAD operations (see README).\n\n")
    fo.write(head + "\n" + "\n".join(rows) + "\n\n")
    fo.write(f"Total steady cutting about {total / 60:.1f} min (plus rapids back, about the same again).\n")
print("\n".join(rows))
print(f"band wall ends at x {band_wall:+.2f}; total {total:.0f} s")

# the cross-section
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

fig, ax = plt.subplots(figsize=(11, 7.5))
ax.add_patch(Rectangle((-BAR / 2, 0), BAR, TOP, fc="#e4e7ec", ec="#555", lw=1))
ax.add_patch(Rectangle((-11.5, 3), 23, 16, fc="#9aa9ba", ec="#333", lw=1.2))
ax.text(0, 11, "gauge\n23 x 16", ha="center", va="center", fontsize=11)
ax.add_patch(Rectangle((-BAR / 2, 0), BAR, 3, fc="#f3d9a6", ec="#a07a2a", lw=0.8))
ax.text(0, 1.5, "carrier 3 (op 2 faces it off)", ha="center", va="center", fontsize=8)
for s in (-1, 1):
    ax.add_patch(Rectangle((BAR / 2 if s > 0 else -BAR / 2 - 2.0, 0), 2.0, 4.7, fc="#777", ec="#333"))
ax.text(18.3, 2.3, "jaws\ngrip 4.7", fontsize=8, va="center")
cmap = plt.get_cmap("viridis")
for pid, what, w, d, f, rpm, xw, zf, side in passes:
    if w == 0:
        continue
    m = predict(w, d, f, rpm)
    c = cmap(min(1, max(0, (m["pct"] - 20) / 60)))
    ax.add_patch(Rectangle((xw, zf), w, d, fc=c, ec="k", lw=0.5, alpha=0.9))
    ax.text(xw + w / 2, zf + d / 2, pid, ha="center", va="center", fontsize=6.5, rotation=90, color="w" if m["pct"] < 50 else "k")
ax.add_patch(Rectangle((xn, 19), band_wall - xn, TOP - 19, fc="none", ec="#c33", lw=1, ls="--"))
ax.text((band_wall + xn) / 2, 25, "faced off\n(adaptive / face)", ha="center", fontsize=8, color="#c33")
ax.set_xlim(-19, 20)
ax.set_ylim(-1, 34)
ax.set_aspect("equal")
ax.set_xlabel("X (mm, across the bar)")
ax.set_ylabel("Z (mm, op 1)")
ax.set_title("Op 1 cross-section: where each calibration pass cuts (colour = predicted spindle %, 20-80)")
sm = plt.cm.ScalarMappable(cmap=cmap, norm=plt.Normalize(20, 80))
fig.colorbar(sm, ax=ax, fraction=0.03, label="predicted spindle load %")
fig.tight_layout()
fig.savefig("calibration-passes.png", dpi=120)
