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
P_IN, EFF, RATED = 1500.0, 0.43, 24000.0   # 645 W: the measured pull-out at 24k (so % is of pull-out)
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


# cuts that have run fine on this machine (step over, depth, feed at 24k)
PROVEN = {(0.6, 6.0, 2400), (1.0, 6.0, 2400), (0.6, 10.0, 2500)}
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


# --- op 1a v2, after T24 broke in C4 (1.8 x 6 at 2400) on this bar. The +X side already has a 4.75 wide step
# down to z 26 (C1-C3 and most of C4), torn at C4's far end. Jonas's rules: step over 0.3-1.0 (<= 15% of D),
# removal from depth and feed, each pass at most about 10 points of predicted load over the one before, the
# ladder stopped at the first heard droop over the limit (the listen-guard feed hold), not at a number.
x = BAR / 2 - 4.8          # the step's wall (4.75 measured, 4.8 planned: C1-C4)
# 0. clean C4's torn far end with <= 1 mm strips: the outer 0.8 (12.2 .. 13.0) then the inner 1.0 (11.2 .. 12.2),
# each in four levels. Air where C4 did cut; the loads are for a full strip, the worst case at the torn end
for xw, w in ((BAR / 2 - 3.8, 0.8), (BAR / 2 - 4.8, 1.0)):
    for d in (2.0, 3.5, 5.0, 6.0):
        n += 1
        passes.append((f"C{n}", f"clean the torn end of the old C4: {w:g} strip", w, d, 2400, 24000, xw, TOP - d, 1))
# The ladder, in running order. Chips at the proven 0.033 mm a tooth (F2400 at 24k, F = rpm / 10 below);
# each cut twice, back to back, in the same direction (about 6 s of steady cutting for the wall meter).
# +X: 6 deep from the step's wall in; -X: 12 deep from the bar's edge in (each region keeps one depth, so no
# pass ever cuts under an earlier one).
def pair(side, what, w, d, f, rpm):
    for k in range(2):
        (cut if side > 0 else cut_neg)(f"{what} ({k + 1}/2)", w, d, f, rpm)


# 1. the new tool on a cut that already ran fine (the first run's C1: 0.6 x 6 at 2400, 1.8% sag)
pair(1, "baseline: the first run's C1 again, new tool", 0.6, 6.0, 2400, 24000)
# 2. lightest to heaviest by the model's watts and by metal removal rate
pair(-1, "12 deep", 0.2, 12.0, 2400, 24000)
pair(-1, "12 deep", 0.3, 12.0, 2400, 24000)
# 3. rpm sweep from 24k down, the same 0.3 x 12 and the same chip: watts fall with the rpm, so with constant
# torque each is the same 53% of what the spindle has at that speed (and at most about 60% of it)
for rpm in (21000, 18000, 15000, 12000):
    pair(-1, f"rpm sweep {rpm // 1000}k", 0.3, 12.0, rpm // 10, rpm)
pair(1, "6 deep", 0.8, 6.0, 2400, 24000)
pair(-1, "12 deep", 0.4, 12.0, 2400, 24000)
# a deliberate comparison: the thin chip (half the proven feed per tooth), labelled as such
pair(-1, "THIN-CHIP COMPARISON 12 deep at half the chip", 0.6, 12.0, 1200, 24000)
pair(1, "6 deep (bog guard armed)", 1.0, 6.0, 2400, 24000)
pair(-1, "12 deep (heaviest, bog guard armed)", 0.45, 12.0, 2400, 24000)
band_wall = x

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
        f"| {pid} | {what} | {w:g} | {d:g} | {f} | {rpm} | {w * d * f / 1000:.1f} | **{m['p']:.0f} W** ({m['pct']:.0f}% of pull-out) | {m['hm'] * 1000:.1f} µm | {t:.1f} | {'proven' if (w, d, f) in PROVEN else ('guard' if m['p'] > 500 else '')} | | x {xw:+.2f} .. {xw + w:+.2f}, z {zf:.1f} |")

head = """| Pass | What | Step over (mm) | Depth (mm) | Feed (mm/min) | rpm | MRR (cm³/min) | Model | Mean chip | Steady cut (s) | Proven? | Wall W (fill in) | Where (op 1 frame) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|"""
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
