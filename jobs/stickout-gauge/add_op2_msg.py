"""Op 2: a stop before Find bores to clean the lug bores, and probe them in the lugs' middle (their top is the
bar's sawn-flat bottom at Z19 here; rCAD writes the gauge's bottom, Z16). Run after posting op2-bottom.nc."""
import sys
p = sys.argv[1]
s = open(p).read()
mark = "(Probe lug bore A - Probe)\n"
stop = """(MSG,Before Find bores: blow the chips out of both lug bores and deburr their upper ends [where they broke through in op 1], then press Cycle Start)
M0

"""
assert mark in s and "MSG,Before Find bores" not in s
s = s.replace(mark, stop + mark, 1)
assert s.count("TOP=16 DEPTH=3") == 1
s = s.replace("TOP=16 DEPTH=3", "TOP=19 DEPTH=4")
open(p, "w").write(s)
print("added")
