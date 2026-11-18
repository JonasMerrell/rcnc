"""Op 2: a stop before probing to clean the lug bores, Find stock for a coarse X/Y, then Find bores in the lugs'
middle for the exact X/Y (their top is the bar's sawn-flat bottom at Z19 here; rCAD writes the gauge's bottom,
Z16). Run after posting op2-bottom.nc."""
import sys
p = sys.argv[1]
s = open(p).read()
mark = "(Probe lug bore A - Probe)\n"
stop = """(MSG,Before probing: blow the chips out of both lug bores and deburr their upper ends [where they broke through in op 1], then press Cycle Start)
M0

"""
assert mark in s and "MSG,Before probing" not in s
s = s.replace(mark, stop + mark, 1)
# Find stock first (Jonas: op 2 starts with Find stock): op 1's G54 doesn't hold for the flipped part, and Find
# bores needs X/Y within a few mm. What op 1 leaves up here: the bar's full outline (31.9 x 120.45 as measured)
# from Z13.5 to its top at Z19 (op 1 cut the sides only down to its Z5.5, and kept the carrier and the lugs at
# full width), so its sides and sawn ends are touched 3 below the top (Z16), well clear of the jaws (Z4.7).
probe = "T99 M6\n"
fs = """(rcnc Find stock: coarse X/Y from the bar's outline left above the jaws [31.9 x 119 as measured by Jonas after op 1, Z13.5 to Z19], sides and ends touched at Z16; Z stays on the Vise reference. Jog the probe over the middle of the part, a few mm above it)
(RCNC FIND_STOCK SIZE=31.9,119,19 AT=0,0,19 DEPTH=3 TOL=1.5 KEEPZ=1)
"""
i = s.index(mark)
j = s.index(probe, i) + len(probe)
assert "FIND_BORES" in s[j:j + 300] and "FIND_STOCK" not in s
s = s[:j] + fs + s[j:]
assert s.count("TOP=16 DEPTH=3") == 1
s = s.replace("TOP=16 DEPTH=3", "TOP=19 DEPTH=4")
open(p, "w").write(s)
print("added")
