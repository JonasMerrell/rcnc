"""Independent check of the T44 finish: a z-map (dexel) simulation of the program's own T44 moves over one car,
compared with the approved surface. Start: the approved surface + START (what the roughing leaves, at most);
every T44 move (G0/G1, linearised every STEP) stamps the tapered ball's lower envelope (R1 tip, 2.24 deg per side).
Prints the largest deviation (cut - approved) over the car top (z >= 3.9, away from the outline) and writes a map.
Usage: ballsim.py program.nc [car_cy (work Y of the car centre, default -26.5)] [work_offset G55]"""
import numpy as np, re, sys, struct
nc = sys.argv[1]; CY = float(sys.argv[2]) if len(sys.argv) > 2 else -26.5; WO = sys.argv[3] if len(sys.argv) > 3 else "G55"
Z2, R, A = 1.67, 1.0, np.radians(2.24)
G, STEP, START = 0.025, 0.02, 0.3
# design height map on the grid, from the turned STL (vise axes, car at the origin)
b = open("/home/jonas/dmc2/eclipse-car/car.stl", "rb").read()
dt = np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]); T = np.frombuffer(b[84:], dtype=dt)["v"].astype(np.float64)
n = np.cross(T[:, 1] - T[:, 0], T[:, 2] - T[:, 0]); T = T[n[:, 2] > 1e-12]
xs = np.arange(-8.65, 8.65 + 1e-9, G); ys = np.arange(-22.5, 22.5 + 1e-9, G)
D = np.full((len(xs), len(ys)), -1.0)
for t in T:
    i0, i1 = np.searchsorted(xs, t[:, 0].min() - 1e-9), np.searchsorted(xs, t[:, 0].max() + 1e-9)
    j0, j1 = np.searchsorted(ys, t[:, 1].min() - 1e-9), np.searchsorted(ys, t[:, 1].max() + 1e-9)
    if i1 <= i0 or j1 <= j0:
        continue
    X, Y = np.meshgrid(xs[i0:i1], ys[j0:j1], indexing="ij")
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = t
    d = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2)
    l0 = ((y1 - y2) * (X - x2) + (x2 - x1) * (Y - y2)) / d; l1 = ((y2 - y0) * (X - x2) + (x0 - x2) * (Y - y2)) / d; l2 = 1 - l0 - l1
    m = (l0 >= -1e-9) & (l1 >= -1e-9) & (l2 >= -1e-9)
    D[i0:i1, j0:j1] = np.maximum(D[i0:i1, j0:j1], np.where(m, l0 * z0 + l1 * z1 + l2 * z2, -1))
S = np.where(D > 0, D + START, -1.0)            # stock over the car top (work: + Z2 later)
# tool envelope kernel
rk = 1.3; k = int(np.ceil(rk / G)); u = np.arange(-k, k + 1) * G
dd = np.hypot(*np.meshgrid(u, u, indexing="ij")); dc = R * np.cos(A)
F = np.where(dd <= dc, R - np.sqrt(np.maximum(R * R - dd * dd, 0)), (R - np.sqrt(R * R - dc * dc)) + (dd - dc) / np.tan(A))
# the program's T44 moves
pts, tool, wo, pos, mode = [], None, None, [0.0, 0.0, 50.0], 0
for line in open(nc):
    s = re.sub(r"\(.*?\)", "", line).strip().upper()
    if not s:
        continue
    m = re.search(r"T(\d+)\s*M6", s)
    if m:
        tool = int(m.group(1)); continue
    if s in ("G54", "G55"):
        wo = s
    gs = re.findall(r"G(\d+)", s)
    if "53" in gs:                     # G53 G0 Z0: machine-coordinate retract, up out of the way
        pos = [pos[0], pos[1], 50.0]; mode = 0; continue
    for g in gs:
        if g in ("0", "00", "1", "01", "2", "02", "3", "03"):
            mode = int(g)
    new = pos[:]
    for ax, i in (("X", 0), ("Y", 1), ("Z", 2)):
        mm = re.search(ax + r"(-?\d*\.?\d+)", s)
        if mm:
            new[i] = float(mm.group(1))
    if new != pos and tool == 44 and wo == WO:
        a, bb = np.array(pos), np.array(new)
        if mode in (2, 3):              # G17 arc, I J incremental from the start (G91.1)
            I = float(re.search(r"I(-?\d*\.?\d+)", s).group(1)) if "I" in s else 0.0
            J = float(re.search(r"J(-?\d*\.?\d+)", s).group(1)) if "J" in s else 0.0
            c = a[:2] + [I, J]; r = np.hypot(I, J)
            a0 = np.arctan2(a[1] - c[1], a[0] - c[0]); a1 = np.arctan2(bb[1] - c[1], bb[0] - c[0])
            if mode == 2:
                while a1 >= a0 - 1e-12: a1 -= 2 * np.pi
            else:
                while a1 <= a0 + 1e-12: a1 += 2 * np.pi
            nseg = max(2, int(abs(a1 - a0) * r / STEP))
            t = np.linspace(0, 1, nseg + 1); ang = a0 + (a1 - a0) * t
            pts.append(np.stack([c[0] + r * np.cos(ang), c[1] + r * np.sin(ang), a[2] + (bb[2] - a[2]) * t], -1))
        else:
            L = np.linalg.norm(bb - a); nseg = max(1, int(L / STEP))
            pts.append(a + (bb - a) * np.linspace(0, 1, nseg + 1)[:, None])
    pos = new
P = np.concatenate(pts); P[:, 1] -= CY; P[:, 2] -= Z2
P = P[(abs(P[:, 0]) < 8.65 + rk) & (abs(P[:, 1]) < 22.5 + rk) & (P[:, 2] < 10)]
print(len(P), "T44 points over the car")
nx, ny = len(xs), len(ys)
for x, y, z in P:
    i, j = int(round((x - xs[0]) / G)), int(round((y - ys[0]) / G))
    a0, a1, b0, b1 = max(i - k, 0), min(i + k + 1, nx), max(j - k, 0), min(j + k + 1, ny)
    if a0 >= a1 or b0 >= b1:
        continue
    sub = S[a0:a1, b0:b1]
    np.minimum(sub, z + F[a0 - i + k:a1 - i + k, b0 - j + k:b1 - j + k], out=sub)
E = S - D
gx, gy = np.gradient(np.where(D > 0, D, np.nan), G)
E = E / np.sqrt(1 + gx * gx + gy * gy)          # along the surface normal: on near-vertical faces a vertical
                                                # difference is mostly a sideways one (the lamp panel, 85 deg)
untouched = np.isclose(S, D + START)            # never reached by T44 (finished by an end mill op, or left)
ins = D >= 3.9 - 1e-9
Pp = np.pad(ins, 4); inner = ins.copy()
for di in range(-4, 5):
    for dj in range(-4, 5):
        inner &= Pp[4 + di:4 + di + nx, 4 + dj:4 + dj + ny]
e = E[inner]
U = untouched & inner
print(f"T44 never touches {U.sum() * G * G:.1f} mm2 of the car top; where it does:")
t = E[inner & ~untouched]
print(f"  cut - approved: max +{t.max():.3f} / min {t.min():.3f}; mean {t.mean():.4f}; 99% within {np.percentile(abs(t), 99):.3f}; 99.9% {np.percentile(abs(t), 99.9):.3f}")
for thr in (0.02, 0.05, 0.1):
    print(f"  touched area with more than {thr} left: {((E > thr) & inner & ~untouched).sum() * G * G:.1f} mm2")
np.save("ballsim-untouched.npy", U)
print(f"car top (z>=3.9, 0.1 in from the outline): {inner.sum()} cells; cut - approved: max +{e.max():.3f} (left on) / min {e.min():.3f} (into it); "
      f"mean {e.mean():.4f}; 99% within {np.percentile(abs(e), 99):.3f}; 99.9% {np.percentile(abs(e), 99.9):.3f}")
for thr in (0.02, 0.05, 0.1, 0.2):
    print(f"  area with more than {thr} left: {(e > thr).sum() * G * G:.1f} mm2")
I = np.unravel_index(np.argmax(np.where(inner, E, -9)), E.shape); print("most left at car x", xs[I[0]].round(2), "y", ys[I[1]].round(2), "z", D[I].round(2))
np.save("ballsim-cut.npy", np.where(D > 0, S, np.nan)); np.save("ballsim-dev.npy", np.where(inner, E, np.nan)); np.save("ballsim-design.npy", np.where(D > 0, D, np.nan))
E2 = np.where(inner, E, np.nan)
import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
fig = plt.figure(figsize=(14, 5)); a = fig.add_subplot(111)
im = a.imshow(E2[:, ::-1], origin="lower", extent=[-22.5, 22.5, -8.65, 8.65], cmap="RdBu_r", vmin=-0.1, vmax=0.1)
a.set_xlabel("along the car, mm (nose at left)"); a.set_ylabel("across, mm (X)"); fig.colorbar(im, label="cut minus approved surface, mm (red = brass left)")
a.set_title(f"{nc.split('/')[-1]}: simulated T44 finish vs the approved surface (0.025 mm z-map)")
fig.tight_layout(); fig.savefig(nc.rsplit(".", 1)[0] + "-ballsim.png", dpi=110)
