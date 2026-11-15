"""Phone stand top surface: a heightfield z(x, y) on the 74 x 74 footprint (op 2 frame: Z0 = part bottom,
y = -37 front .. +37 back). Writes stand-top.stl (closed solid: heightfield top, walls, flat bottom)."""
import math, struct, sys
import numpy as np

W = 74.0          # footprint, square, corner radius RC
RC = 10.0
FLOOR = 10.0      # groove floor
LIP_Y = -27.0     # lip's inner face
LIP_TOP = 17.0
WALL0 = (-12.0, FLOOR)   # backrest foot
WALL1 = (-11.0, 26.5)    # direction of the backrest face (steep)
TOP = 29.5
CREST_R = 3.0
TAIL_Y0, TAIL_Y1, TAIL_Z1 = -6.0, 37.0, 14.5
PLINTH = 13.0     # walls finished in op 1 up to here; the top rolls into it
ROLL = 6.0

def crest_arc():
    # convex arc tangent to the backrest line and to z = TOP
    (x0, z0), (x1, z1) = WALL0, WALL1
    d = np.array([x1 - x0, z1 - z0]); d /= np.linalg.norm(d)
    n = np.array([d[1], -d[0]])                  # points back (+y) and down
    cz = TOP - CREST_R
    # centre c with (c - p0) . n = CREST_R (inside, behind the face)
    t = (cz - z0 - CREST_R * n[1]) / d[1]
    c = np.array([x0, z0]) + t * d + CREST_R * n
    tan_wall = c - CREST_R * n
    return c, tan_wall

C, TW = crest_arc()

def profile(y):
    """Centre-line height (no crown, no edge roll)."""
    if y < LIP_Y - 1.5:
        return LIP_TOP
    if y < LIP_Y:
        return LIP_TOP - 1.5 + math.sqrt(max(0, 1.5 ** 2 - (y - (LIP_Y - 1.5)) ** 2))
    if y < WALL0[0]:
        return FLOOR
    if y < TW[0]:
        return FLOOR + (y - WALL0[0]) * (TW[1] - FLOOR) / (TW[0] - WALL0[0])
    if y < C[0]:
        return C[1] + math.sqrt(max(0, CREST_R ** 2 - (y - C[0]) ** 2))
    if y < TAIL_Y0:
        return TOP
    s = min(1.0, (y - TAIL_Y0) / (TAIL_Y1 - TAIL_Y0))
    return TAIL_Z1 + (TOP - TAIL_Z1) * (1 + math.cos(math.pi * s)) / 2

def inside_dist(x, y):
    """Distance inside the rounded-square footprint (negative outside)."""
    h = W / 2 - RC
    qx, qy = abs(x) - h, abs(y) - h
    if qx > 0 and qy > 0:
        return RC - math.hypot(qx, qy)
    return W / 2 - max(abs(x), abs(y))

def height(x, y, py=None):
    h = profile(y) if py is None else py
    if h > FLOOR:
        q = x / (W / 2); h = FLOOR + (h - FLOOR) * (1 - 0.3 * q ** 2 - 0.2 * q ** 6)
    d = inside_dist(x, y)
    if h > PLINTH and d < ROLL:
        f = max(0.0, d) / ROLL
        h = PLINTH + (h - PLINTH) * math.sqrt(max(0.0, 1 - (1 - f) ** 2))
    return h

def grid():
    xs = np.linspace(-W / 2, W / 2, 149)
    ys = set(np.round(np.linspace(-W / 2, W / 2, 149), 4))
    # exact breakpoints, the lip face as a step
    ys |= {LIP_Y - 1.5, LIP_Y - 0.05, WALL0[0], TW[0], C[0], TAIL_Y0}
    ys |= set(np.round(np.linspace(LIP_Y - 1.5, LIP_Y - 0.05, 6), 4))
    ys |= set(np.round(np.linspace(TW[0], C[0], 8), 4))
    ys = np.array(sorted(ys))
    return xs, ys

def warp(u, v):
    """Square grid -> rounded square: the corner squares map onto quarter discs."""
    a = W / 2 - RC
    s, t = (abs(u) - a) / RC, (abs(v) - a) / RC
    if s <= 0 or t <= 0:
        return u, v
    x = s * math.sqrt(1 - t * t / 2)
    y = t * math.sqrt(1 - s * s / 2)
    return math.copysign(a + RC * x, u), math.copysign(a + RC * y, v)

def write_stl(path):
    xs, ys = grid()
    ny, nx = len(ys), len(xs)
    PT = [[None] * nx for _ in range(ny)]
    for i in range(ny):
        for j in range(nx):
            x, y = warp(xs[j], ys[i])
            PT[i][j] = (x, y, height(x, y))
    tris = []
    def quad(a, b, c, d):
        tris.append((a, b, c)); tris.append((a, c, d))
    P = lambda i, j: PT[i][j]
    B = lambda p: (p[0], p[1], 0.0)
    for i in range(ny - 1):
        for j in range(nx - 1):
            quad(P(i, j), P(i, j + 1), P(i + 1, j + 1), P(i + 1, j))
    # boundary loop, counter-clockwise seen from above
    loop = [P(0, j) for j in range(nx - 1)] + [P(i, nx - 1) for i in range(ny - 1)] \
         + [P(ny - 1, j) for j in range(nx - 1, 0, -1)] + [P(i, 0) for i in range(ny - 1, 0, -1)]
    n = len(loop)
    for k in range(n):
        a, b = loop[k], loop[(k + 1) % n]
        quad(B(a), B(b), b, a)
    c = (0.0, 0.0, 0.0)
    for k in range(n):
        a, b = loop[k], loop[(k + 1) % n]
        tris.append((c, B(b), B(a)))
    with open(path, "wb") as f:
        f.write(b"\0" * 80 + struct.pack("<I", len(tris)))
        for t in tris:
            a, b, cc = (np.array(v) for v in t)
            nn = np.cross(b - a, cc - a); nn = nn / (np.linalg.norm(nn) or 1)
            f.write(struct.pack("<12fH", *nn, *a, *b, *cc, 0))
    return len(tris)

def lean(t):
    """Phone of thickness t: bottom pushed against the lip, back on the crest. Lean from vertical, deg."""
    best = None
    for k in range(0, 4501, 10):
        th = math.radians(k / 100)
        p0 = np.array([LIP_Y + t * math.cos(th), FLOOR])      # back-bottom corner on the floor
        u = np.array([math.sin(th), math.cos(th)])
        # the line must stay in front of (not inside) the profile
        for s in np.linspace(0.5, 160, 1600):
            q = p0 + s * u
            if q[1] < height(0, q[0]) + 1e-6:
                return k / 100
    return None

if __name__ == "__main__":
    print("crest centre", C, "tangent", TW)
    print("tris", write_stl(sys.argv[1] if len(sys.argv) > 1 else "stand-top.stl"))
    for t in (8, 10, 12):
        print("t", t, "lean", lean(t))
