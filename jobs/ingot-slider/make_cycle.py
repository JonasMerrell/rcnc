"""Merge a material's op 1 (left station, G54) and op 2 (+ engraving) (right station, G55) programs into one
two-station cycle program, ordered by tool: each cutter is loaded and measured once and cuts in both stations.

usage: make_cycle.py OUT.nc LEFT.nc RIGHT.nc [RIGHT2.nc ...]

Each input is an rCAD rcnc post: header comments, `G21 ...`, `G54`/`G55`, an optional `(RCNC Z_REFERENCE ...)`,
then tool blocks that start with an operation comment, `G53 G0 Z0` and `Tn M6`. Blocks are grouped by tool in the
order the tools first appear; a station's later block for a tool that is already loaded keeps everything but the
`Tn M6` and starts with `G53 G0 Z0` and its work offset. rcnc (PR 14) zeroes every tool in both work offsets.
"""
import re
import sys

out_path, inputs = sys.argv[1], sys.argv[2:]


def parse(path):
    lines = open(path).read().splitlines()
    g21 = next(i for i, l in enumerate(lines) if l.startswith("G21 "))
    header = lines[:g21]
    m6 = [i for i, l in enumerate(lines) if re.fullmatch(r"T\d+ M6", l)]
    # a block starts at the operation comment before the G53 G0 Z0 that precedes its M6
    starts = []
    for i in m6:
        j = i - 1
        while lines[j] in ("M5", "M9"):
            j -= 1
        assert lines[j] == "G53 G0 Z0", (path, lines[j])
        j -= 1
        while j > g21 and lines[j].startswith("("):
            j -= 1
        starts.append(j + 1)
    pre = [l for l in lines[g21 + 1:starts[0]] if l.strip()]
    wcs = next(l for l in pre if re.fullmatch(r"G5[4-9]", l))
    zref = [l for l in pre if l.startswith("(RCNC Z_REFERENCE")]
    end = max(i for i, l in enumerate(lines) if l == "G53 G0 Z0")
    assert lines[end + 1:] [-1] == "M30", path
    blocks = []
    for k, s in enumerate(starts):
        e = starts[k + 1] if k + 1 < len(starts) else end
        tool = int(re.fullmatch(r"T(\d+) M6", lines[m6[k]]).group(1))
        blocks.append((tool, lines[s:e]))
    return {"path": path, "header": header, "g21": lines[g21], "wcs": wcs, "zref": zref, "blocks": blocks}


progs = [parse(p) for p in inputs]
order = []
for p in progs:
    for t, _ in p["blocks"]:
        if t not in order:
            order.append(t)

tools = []
for p in progs:
    for l in p["header"]:
        if re.match(r"\(T\d+ ", l) and l not in tools:
            tools.append(l)
out = [f"(Two-station cycle - made by make_cycle.py from {', '.join(p['path'] for p in progs)})",
       "(LEFT station, G54: op 1 on a raw blank. RIGHT station, G55: op 2 on the blank flipped from the last cycle)",
       "(retracts use G53 G0 Z0 - home the machine first; needs rcnc with per-work-offset Z references [PR 14])"]
out += tools + [progs[0]["g21"]]
loaded = None
named = set()          # work offsets whose Z reference has been given
for t in order:
    for p in progs:
        for tool, blk in p["blocks"]:
            if tool != t:
                continue
            body = list(blk)
            out.append("")
            out.append(f"(=== {p['wcs']} {'LEFT' if p['wcs'] == 'G54' else 'RIGHT'} station: {p['path']} ===)")
            if loaded == t:
                # the tool is in: drop its change, go up and over to this station
                i = body.index(f"T{t} M6")
                j = i
                while body[j - 1] in ("M5", "M9"):
                    j -= 1
                body = body[:j] + body[i + 1:]
            # the work offset (and its Z reference) before the block's first move
            g53 = body.index("G53 G0 Z0")
            ins = [p["wcs"]] + ([] if p["wcs"] in named else p["zref"])
            named.add(p["wcs"])
            if loaded == t:
                body[g53 + 1:g53 + 1] = ins
            else:
                body[g53:g53] = ins
            out += body
            loaded = t
out += ["", "G53 G0 Z0", "M5", "M9", "M30"]
open(out_path, "w").write("\n".join(out) + "\n")
print(out_path, "tool changes:", len(order), "blocks:", sum(len(p["blocks"]) for p in progs))
