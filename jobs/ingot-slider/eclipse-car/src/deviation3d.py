"""3D deviation of a mesh from the approved height map: every map point inside the outline (x, y, z_map) to the
nearest point of the mesh (trimesh closest point). Vertical differences mean little on near-vertical faces (the
0.15 STL and the 0.1 map sample a cliff at different places), so the distance to the surface is the measure.
Usage: deviation3d.py mesh.stl [turned]"""
import numpy as np, sys, trimesh
MAP = "/home/jonas/src/rcnc-eclipse/jobs/ingot-slider/eclipse-design/eclipse-car-heightmap-0.1mm.csv"
A = np.genfromtxt(MAP, delimiter=","); ys = A[0, 1:]; xs = A[1:, 0]; Z = A[1:, 1:]
m = trimesh.load(sys.argv[1], force="mesh")
if len(sys.argv) > 2:
    v = m.vertices.copy(); m.vertices = np.stack([-v[:, 1], v[:, 0], v[:, 2]], -1)
X, Y = np.meshgrid(xs, ys, indexing="ij"); ins = Z >= 3.9 - 1e-9
P = np.stack([X[ins], Y[ins], Z[ins]], -1)
from scipy.spatial import cKDTree
from trimesh.triangles import closest_point as tcp
tri = m.triangles; tree = cKDTree(tri.mean(1)); K = int(__import__("os").environ.get("DEVK", "24"))
_, cand = tree.query(P, K)
d = np.full(len(P), np.inf)
for k in range(K):
    c = tcp(tri[cand[:, k]], P); d = np.minimum(d, np.linalg.norm(c - P, axis=1))
k = np.argmax(d)
print(f"{sys.argv[1]}: {len(P)} map points; max {d.max():.4f} mm at x {P[k,0]:.2f} y {P[k,1]:.2f} z {P[k,2]:.2f}; mean {d.mean():.4f}; 99% {np.percentile(d,99):.4f}; 99.9% {np.percentile(d,99.9):.4f}")
