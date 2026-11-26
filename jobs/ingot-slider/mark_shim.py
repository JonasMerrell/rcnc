"""mark_shim.py file.nc: an (RCNC REQUIRE_SHIM: …) line per station that stands its work on shims (rCAD's
"fixture … shim …" header lines), for the stations the program really runs (its G54 / G55 …). rcnc (shim-check
build) then asks "Shims in?" at Cycle Start. 2026-11-26: the car's op 1 ran without its shims and cut the jaws."""
import re, sys
p = sys.argv[1]
lines = open(p).read().split("\n")
if any(l.upper().startswith("(RCNC REQUIRE_SHIM") for l in lines):
    sys.exit(0)
code = " ".join(re.sub(r"\(.*?\)", "", l) for l in lines)
used = set(re.findall(r"\bG5[4-9]\b", code.upper()))
SIDE = {"G54": "left station", "G55": "right station"}
stations, cur = [], None
for l in lines:
    m = re.match(r"\(station \d+: (G5\d) (.*?) - ", l)
    if m:
        cur = {"g": m.group(1), "name": m.group(2), "shims": []}; stations.append(cur); continue
    if re.match(r"\(\s*fixture .*shim", l, re.I):
        if cur is None:
            cur = {"g": (sorted(used) or ["G54"])[0], "name": lines[0].strip("()").split(" - ")[0], "shims": []}; stations.append(cur)
        cur["shims"].append(re.match(r"\(\s*fixture (.*?)(?:, op \d)?:", l).group(1))
out = []
for s in stations:
    if s["shims"] and s["g"] in used:
        out.append(f"(RCNC REQUIRE_SHIM: {SIDE.get(s['g'], s['g'])} [{s['g']}, {s['name']}]: " + "; ".join(s["shims"]) + ")")
if out:
    i = next(k for k, l in enumerate(lines) if l.upper().startswith("(RCNC REQUIRE_GUARD")) + 1 if any(l.upper().startswith("(RCNC REQUIRE_GUARD") for l in lines) else 1
    lines[i:i] = out
    open(p, "w").write("\n".join(lines))
print(p, len(out), "shim line(s)")
