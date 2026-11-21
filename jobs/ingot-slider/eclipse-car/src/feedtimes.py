"""Per-tool feed length and the program time at other T44 feeds: rCAD's cycle time with the T44 cutting moves
re-timed (length / feed, the machine's 3000 cap). Plunges (F300) and rapids unchanged.
Usage: feedtimes.py prog.nc rcad_cycle_s [feeds...]"""
import re, sys, math
p, total = sys.argv[1], float(sys.argv[2]); feeds = [float(f) for f in sys.argv[3:]] or [1000, 2000]
tool, mode, F, pos = None, 0, 1000.0, [0.0, 0.0, 50.0]
L = {}   # (tool, feed) -> mm
for line in open(p):
    s = re.sub(r"\(.*?\)", "", line).strip().upper()
    if not s:
        continue
    m = re.search(r"T(\d+)\s*M6", s)
    if m:
        tool = int(m.group(1)); continue
    gs = re.findall(r"G(\d+)", s)
    if "53" in gs:
        pos[2] = 50.0; continue
    for g in gs:
        if g in ("0", "00", "1", "01", "2", "02", "3", "03"):
            mode = int(g)
    f = re.search(r"F([\d.]+)", s)
    if f:
        F = float(f.group(1))
    new = pos[:]
    for i, a in enumerate("XYZ"):
        v = re.search(a + r"(-?[\d.]+)", s)
        if v:
            new[i] = float(v.group(1))
    if mode in (1, 2, 3) and new != pos:
        L[(tool, F)] = L.get((tool, F), 0) + math.dist(pos, new)
    pos = new
for (t, f), l in sorted(L.items(), key=lambda kv: (kv[0][0] or 0, kv[0][1])):
    print(f"T{t} F{f:g}: {l / 1000:.2f} m feed, {l / f:.1f} min at that feed")
base = sum(l / f for (t, f), l in L.items() if t == 44 and f > 400)
print(f"T44 cutting at the planned feed: {base:.1f} min of the {total / 60:.1f} min program")
for nf in feeds:
    new = sum(l / min(nf * f / 1500, 3000) for (t, f), l in L.items() if t == 44 and f > 400)
    print(f"  T44 at F{nf:g}: program about {(total / 60 - base + new):.1f} min")
