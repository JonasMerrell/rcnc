"""Clean the approved STL into a closed manifold: it carries 785 pairs of opposite-facing triangles on z 0
(top-surface cells outside the outline lying on the bottom, cancelling it). Remove both of each pair."""
import numpy as np, struct, sys
src, dst = sys.argv[1], sys.argv[2]
b = open(src, "rb").read()
dt = np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]); a = np.frombuffer(b[84:], dtype=dt)
v = a["v"].astype(np.float64)
k = np.round(v.reshape(-1, 3) / 1e-6).astype(np.int64)
u, inv = np.unique(k, axis=0, return_inverse=True); inv = inv.reshape(-1, 3); P = u * 1e-6
t = np.sort(inv, 1); ut, back, ct = np.unique(t, axis=0, return_inverse=True, return_counts=True)
keep = ct[back.ravel()] == 1
inv = inv[keep]; print("removed", (~keep).sum(), "kept", keep.sum())
e = np.concatenate([inv[:, [0, 1]], inv[:, [1, 2]], inv[:, [2, 0]]])
s = np.sort(e, 1); _, c = np.unique(s, axis=0, return_counts=True)
_, cd = np.unique(e, axis=0, return_counts=True)
print("edge uses", np.bincount(c), "dup directed", (cd > 1).sum())
tris = P[inv]
vol = np.einsum("ij,ij->i", tris[:, 0], np.cross(tris[:, 1], tris[:, 2])).sum() / 6
print("volume", vol)
out = np.zeros(len(tris), dtype=dt); out["v"] = tris.astype(np.float32)
nn = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0]); nn /= np.linalg.norm(nn, axis=1, keepdims=True); out["n"] = nn
open(dst, "wb").write(b[:80] + struct.pack("<I", len(tris)) + out.tobytes())
