"""Op 1's wall contour against the true plan outline (45 x 17.3, R4.8 nose, R2.4 tail): the tool centre should run
3.175 outside it everywhere. Prints the spread of (distance - 3.175) per car, with the nose at either end."""
import re, sys, numpy as np
p = sys.argv[1]
lines = open(p).read().split("\n")
i0 = next(i for i, l in enumerate(lines) if l.startswith("(Finish the walls"))
i1 = next(i for i in range(i0 + 1, len(lines)) if lines[i].startswith("(") and "Contour" not in lines[i] and lines[i].endswith(")") and " - " in lines[i])
pos = [0.0, 0.0, 50.0]; mode = 0; pts = []
for l in lines[i0:i1]:
    s = re.sub(r"\(.*?\)", "", l).strip()
    if not s:
        continue
    for g in re.findall(r"G(\d+)", s):
        if g in ("0", "1", "2", "3"):
            mode = int(g)
    new = pos[:]
    for k, a in enumerate("XYZ"):
        m = re.search(a + r"(-?\d*\.?\d+)", s)
        if m:
            new[k] = float(m.group(1))
    if mode in (2, 3) and new != pos:
        I = float(re.search(r"I(-?\d*\.?\d+)", s).group(1)); J = float(re.search(r"J(-?\d*\.?\d+)", s).group(1))
        c = np.array(pos[:2]) + [I, J]; r = np.hypot(I, J)
        a0 = np.arctan2(pos[1] - c[1], pos[0] - c[0]); a1 = np.arctan2(new[1] - c[1], new[0] - c[0])
        if mode == 2:
            while a1 >= a0: a1 -= 2 * np.pi
        else:
            while a1 <= a0: a1 += 2 * np.pi
        for t in np.linspace(0, 1, 40):
            pts.append((c[0] + r * np.cos(a0 + (a1 - a0) * t), c[1] + r * np.sin(a0 + (a1 - a0) * t), new[2]))
    elif mode == 1 and new != pos:
        for t in np.linspace(0, 1, max(2, int(np.hypot(new[0] - pos[0], new[1] - pos[1]) / 0.05))):
            pts.append(tuple(np.array(pos) + (np.array(new) - np.array(pos)) * t))
    pos = new
P = np.array(pts); P = P[P[:, 2] < 4]     # at full depth
def sd(x, y, nose_pos):
    hx, hy = 8.65, 22.5; r = np.where((y > 0) == nose_pos, 4.8, 2.4)
    qx = abs(x) - (hx - r); qy = abs(y) - (hy - r)
    return np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - r
for cy in (-26.5, 26.5):
    Q = P[abs(P[:, 1] - cy) < 30]
    Q = Q[abs(Q[:, 1] - cy) < 22.5 + 3.3]
    for nose_pos in (True, False):
        d = sd(Q[:, 0], Q[:, 1] - cy, nose_pos) - 3.175
        print(f"car at y {cy:+}: nose toward {'+' if nose_pos else '-'}Y: tool centre minus 3.175 from the true outline: min {d.min():+.3f} max {d.max():+.3f}")
for cy in (-26.5, 26.5):
    Q = P[abs(P[:, 1] - cy) < 22.5 + 3.3]
    d = sd(Q[:, 0], Q[:, 1] - cy, False) - 3.175
    far = Q[d > 0.02]
    print(f"car {cy:+}: {len(Q)} pts, {(d > 0.02).sum()} more than 0.02 off; 99th pct {np.percentile(d, 99):.3f}; far ones at", np.round(far[::max(1, len(far)//8), :2] - [0, cy], 2).tolist())
# per direction from the car centre (1 deg bins): the closest pass, i.e. the last lap that sets the wall
Q = P[abs(P[:, 1] - 26.5) < 22.5 + 3.3]; q = Q[:, :2] - [0, 26.5]
d = sd(q[:, 0], q[:, 1], False) - 3.175
# bin by the nearest outline point's arc position: use angle around (0, +-13) half-centres to spread the ends
ang = np.degrees(np.arctan2(q[:, 1] - np.clip(q[:, 1], -13, 13), q[:, 0])) + np.where(q[:, 1] > 0, 0, 0)
key = np.round(np.clip(q[:, 1], -13, 13)).astype(int) * 1000 + np.round(ang / 2).astype(int)
best = {}
for k, v in zip(key, d):
    best[k] = min(best.get(k, 9), v)
b = np.array(list(best.values())); print(f"closest pass per bin: min {b.min():+.3f}, max {b.max():+.3f}, 99% {np.percentile(b, 99):+.3f} ({len(b)} bins)")
bad = [k for k, v in best.items() if v > 0.05]
print("bins > 0.05 off (y_clip*1000 + angle/2):", sorted(bad)[:40], len(bad))
