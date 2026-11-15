"""Add a pause to blow the chips out of the groove after op 2's 3D rough (the chip model's caution there).
Run after posting; delete the block between the CHIP PAUSE comments to run without it."""
import sys
p = sys.argv[1]
s = open(p).read()
mark = "(3D semi-rough 1:"
block = """(CHIP PAUSE: the chip model says chips pack in the groove during the rough; delete down to END CHIP PAUSE to skip)
G53 G0 Z0
M5
M9
(MSG,Blow the chips out of the groove AND the two locating bores [the probe finds the bores later; a chip or burr there shifts X/Y], spindle and coolant are off; then press Cycle Start)
M0
S24000 M3
M8
G4 P4
(END CHIP PAUSE)

"""
assert "CHIP PAUSE" not in s and mark in s
i = s.index(mark)
open(p, "w").write(s[:i] + block + s[i:])
print("added")
