"""What an R1 ball can reach at best (morphological opening of the approved surface by the ball), against what the
program's T44 moves leave (ballsim.py's z-map). Cells the ball can't reach are the tool's limit, not the toolpath's."""
import numpy as np
G = 0.025
D = np.load("ballsim-design.npy"); E = np.load("ballsim-dev.npy"); U = np.load("ballsim-untouched.npy")
Dz = np.where(np.isnan(D), 0.0, D)
k = int(round(1.0 / G)); off = [(i, j) for i in range(-k, k + 1) for j in range(-k, k + 1) if (i * i + j * j) * G * G <= 1.0]
F = {o: 1 - np.sqrt(max(1 - (o[0] ** 2 + o[1] ** 2) * G * G, 0)) for o in off}
P = np.pad(Dz, k, constant_values=0.0); nx, ny = Dz.shape
C = np.full(Dz.shape, -9.0)
for (i, j), f in F.items():
    np.maximum(C, P[k + i:k + i + nx, k + j:k + j + ny] - f, out=C)
PC = np.pad(C, k, constant_values=-9.0); O = np.full(Dz.shape, 99.0)
for (i, j), f in F.items():
    np.minimum(O, PC[k + i:k + i + nx, k + j:k + j + ny] + f, out=O)
gx, gy = np.gradient(np.where(np.isnan(D), np.nan, D), G)
lim = (O - Dz) / np.sqrt(1 + gx * gx + gy * gy)
inner = ~np.isnan(E)
for thr in (0.02, 0.05, 0.1):
    print(f"R1 ball limit: {((lim > thr) & inner).sum() * G * G:.1f} mm2 with more than {thr} left (best possible)")
extra = E - lim
t = inner & ~U & (lim > -0.05)
print(f"program vs best possible, where T44 cuts: max {np.nanmax(extra[t]):.3f}, 99.9% {np.nanpercentile(extra[t], 99.9):.3f}, "
      f"area more than 0.02 over the limit {((extra > 0.02) & t).sum() * G * G:.1f} mm2, over 0.05 {((extra > 0.05) & t).sum() * G * G:.1f} mm2")
xs = np.arange(-8.65, 8.65 + 1e-9, G); ys = np.arange(-22.5, 22.5 + 1e-9, G)
I = np.unravel_index(np.nanargmax(np.where(t, extra, np.nan)), E.shape); print("worst over the limit at car x", xs[I[0]].round(2), "y", ys[I[1]].round(2), "z", D[I].round(2))
print(f"untouched by T44: roof (z >= 9.49) {((D >= 9.49) & U).sum() * G * G:.1f} mm2, nose/tail band at 3.9 {((D <= 3.91) & U).sum() * G * G:.1f} mm2, other {((D > 3.91) & (D < 9.49) & U).sum() * G * G:.1f} mm2")
m = (extra > 0.02) & t
ii, jj = np.where(m)
import collections
c = collections.Counter(zip(np.round(xs[ii]).astype(int), np.round(ys[jj]).astype(int), np.round(D[ii, jj], 1)))
print("over the limit by > 0.02 (car x, y, z: cells):", c.most_common(12))
print("of which at the roof z >= 9.4:", ((D >= 9.4) & m).sum() * G * G, "mm2")
nr = inner & (D < 9.4)
for thr in (0.05, 0.1, 0.2):
    print(f"off the roof: program leaves more than {thr} on {((E > thr) & nr).sum() * G * G:.1f} mm2 (R1 limit {((lim > thr) & nr).sum() * G * G:.1f})")
I = np.unravel_index(np.nanargmax(np.where(nr, E, np.nan)), E.shape); print("most left off the roof at car x", xs[I[0]].round(2), "y", ys[I[1]].round(2), "z", D[I].round(2), "left", E[I].round(3), "limit there", lim[I].round(3), "untouched", U[I])
box = (np.abs(xs)[:, None] > 6.3) & (ys[None, :] > 4.0) & (ys[None, :] < 7.5)
mb = box & inner
print("mirror boxes: program max left", np.nanmax(E[mb]).round(3), "R1 limit max", np.nanmax(np.where(lim > -0.05, lim, np.nan)[mb]).round(3),
      "| area > 0.1: program", ((E > 0.1) & mb).sum() * G * G, "limit", ((lim > 0.1) & mb).sum() * G * G)
