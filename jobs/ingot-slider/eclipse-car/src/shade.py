"""Hill-shaded pictures of the simulated T44 cut (ballsim-cut.npy) next to the approved surface, nose at left."""
import numpy as np, matplotlib
matplotlib.use("Agg"); import matplotlib.pyplot as plt
from matplotlib.colors import LightSource
C = np.load("ballsim-cut.npy"); D = np.load("ballsim-design.npy")
U = np.isclose(C, D + 0.3)          # never touched by T44: the roof (faced by T24) and the 3.9 nose band (T24 floor)
C = np.where(U, D, C)
flat = (D <= 3.905) | (D >= 9.495)   # the T24 floor at car z 3.9 (light rough, leave_floor 0) and the faced roof
C = np.where(flat, np.minimum(C, D), C)
ls = LightSource(azdeg=315, altdeg=35)
fig, ax = plt.subplots(2, 1, figsize=(14, 7.4))
for a, Z, t in ((ax[0], D, "approved surface (height map STL, trimmed to the exact outline)"), (ax[1], C, "simulated cut: the program's T44 moves (roof and 3.9 nose band as the T24 face / floor leave them)")):
    Zf = np.where(np.isnan(Z), 3.0, Z)[:, ::-1]
    rgb = ls.shade(Zf, cmap=plt.cm.copper, vert_exag=6, blend_mode="soft", dx=0.025, dy=0.025)
    a.imshow(rgb, origin="lower", extent=[-22.5, 22.5, -8.65, 8.65]); a.set_title(t); a.set_xlabel("along the car, mm (nose at left)")
fig.tight_layout(); fig.savefig("eclipse-car/eclipse-car-op2-cut-vs-approved.png", dpi=110)
