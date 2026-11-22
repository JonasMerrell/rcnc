"""Exact check of T44 ball centres near a focus point: distance from each ball centre (tip + 1) to the car
triangles (exact point-triangle distance), minus R1. Negative = the ball would cut into the part there.
Usage: cornercheck.py prog.nc car_cy focus_x focus_y"""
import re, sys, numpy as np
from trimesh.triangles import closest_point
nc, CY, FX, FY = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4])
b = open("car.stl", "rb").read(); dt = np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")])
T = np.frombuffer(b[84:], dtype=dt)["v"].astype(np.float64).copy(); T[..., 2] += 1.67; T[..., 1] += CY
lo = T.min(1); hi = T.max(1)
T = T[(lo[:, 0] < FX + 2.5) & (hi[:, 0] > FX - 2.5) & (lo[:, 1] < FY + 2.5) & (hi[:, 1] > FY - 2.5)]
tool, mode, pos, C = None, 0, [0, 0, 50], []
for l in open(nc):
    s = re.sub(r"\(.*?\)", "", l).strip()
    m = re.search(r"T(\d+)\s*M6", s)
    if m: tool = int(m.group(1)); continue
    g = re.findall(r"G(\d+)", s)
    if "53" in g: pos = [pos[0], pos[1], 50]; continue
    for x in g:
        if x in ("0", "1", "2", "3"): mode = int(x)
    new = pos[:]
    for k, a in enumerate("XYZ"):
        mm = re.search(a + r"(-?[\d.]+)", s)
        if mm: new[k] = float(mm.group(1))
    if tool == 44 and new != pos and (abs(new[0] - FX) < 3 or abs(pos[0] - FX) < 3) and (abs(new[1] - FY) < 3 or abs(pos[1] - FY) < 3):
        a, bb = np.array(pos), np.array(new)
        if mode in (2, 3):
            I = float(re.search(r"I(-?[\d.]+)", s).group(1)); J = float(re.search(r"J(-?[\d.]+)", s).group(1))
            cc = a[:2] + [I, J]; r = np.hypot(I, J); a0 = np.arctan2(a[1] - cc[1], a[0] - cc[0]); a1 = np.arctan2(bb[1] - cc[1], bb[0] - cc[0])
            if mode == 2:
                while a1 >= a0: a1 -= 2 * np.pi
            else:
                while a1 <= a0: a1 += 2 * np.pi
            t = np.linspace(0, 1, max(2, int(abs(a1 - a0) * r / 0.01)))
            C.append(np.stack([cc[0] + r * np.cos(a0 + (a1 - a0) * t), cc[1] + r * np.sin(a0 + (a1 - a0) * t), a[2] + (bb[2] - a[2]) * t], -1))
        else:
            t = np.linspace(0, 1, max(2, int(np.linalg.norm(bb - a) / 0.01)))
            C.append(a + np.outer(t, bb - a))
    pos = new
C = np.concatenate(C); C = C[(np.abs(C[:, 0] - FX) < 1.0) & (np.abs(C[:, 1] - FY) < 1.0) & (C[:, 2] < 12)]; C[:, 2] += 1.0
print(len(T), "triangles,", len(C), "ball centres")
best = np.full(len(C), np.inf)
for i in range(0, len(T), 200):
    tt = T[i:i + 200]
    for k in range(len(tt)):
        p = closest_point(np.repeat(tt[k:k + 1], len(C), 0), C)
        best = np.minimum(best, np.linalg.norm(p - C, axis=1))
g = best - 1.0; k = np.argmin(g)
print(f"closest: ball {g.min():+.4f} at tip {np.round(C[k] - [0, 0, 1.0], 3)}; centres more than 0.01 in: {(g < -0.01).sum()}, 0.02 in: {(g < -0.02).sum()}")
# the flank above the ball: at each height h above the centre the cutter is 1 + h tan(2.24 deg) wide; the 3D
# distance from the axis point there to the part (<= the horizontal one) must be at least that
tn = np.tan(np.radians(2.24)); H = np.arange(0.1, 3.01, 0.1); Cs = C[::4]
Q = (Cs[:, None, :] + np.stack([np.zeros_like(H), np.zeros_like(H), H], -1)[None]).reshape(-1, 3)
bq = np.full(len(Q), np.inf)
for k in range(len(T)):
    p = closest_point(np.repeat(T[k:k + 1], len(Q), 0), Q); bq = np.minimum(bq, np.linalg.norm(p - Q, axis=1))
need = np.tile(1.0 + H * tn, len(Cs)); g2 = bq - need; k = np.argmin(g2)
print(f"flank: worst {g2.min():+.4f} at tip {np.round(Cs[k // len(H)] - [0, 0, 1.0], 3)}, {H[k % len(H)]:.1f} above the centre; points more than 0.01 in: {(g2 < -0.01).sum()}")
