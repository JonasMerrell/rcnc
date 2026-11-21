"""The approved car STL (rcnc design/eclipse-car-surface 25ebef5) turned for the vise: length along Y, nose at +Y.
Writes car.stl (one car at the origin) and cars-cam.stl (two cars at y -26.5 / +26.5, up 1.67 for op 2's shims)."""
import struct, numpy as np, sys
SRC = "approved-clean.stl"   # clean.py of the approved STL (rcnc design/eclipse-car-surface 25ebef5)
b = open(SRC, "rb").read(); n = struct.unpack("<I", b[80:84])[0]
dt = np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")])
a = np.frombuffer(b[84:], dtype=dt)
v = a["v"].astype(np.float64)
def turned(v):   # (x, y, z) -> (y, -x, z): nose (x -22.5) to +Y, handedness kept
    return np.stack([v[..., 1], -v[..., 0], v[..., 2]], -1)
def write(path, tris):
    out = np.zeros(len(tris), dtype=dt); out["v"] = tris.astype(np.float32)
    e1 = tris[:, 1] - tris[:, 0]; e2 = tris[:, 2] - tris[:, 0]; nn = np.cross(e1, e2)
    nn /= np.maximum(np.linalg.norm(nn, axis=1, keepdims=True), 1e-12); out["n"] = nn
    open(path, "wb").write(b"eclipse car (approved surface, turned: nose +Y)".ljust(80, b" ") + struct.pack("<I", len(tris)) + out.tobytes())
t = turned(v)
write("car.stl", t)
for cy in (-26.5, 26.5):
    pass
two = np.concatenate([t + [0, -26.5, 1.67], t + [0, 26.5, 1.67]])
write("cars-cam.stl", two)
print(n, t.reshape(-1, 3).min(0), t.reshape(-1, 3).max(0))
