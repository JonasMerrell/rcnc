"""Pause after op 1's slot roughing to blow the chips out of the slots and reliefs (the chip model's caution:
6.8-7.2 wide, 6 deep, a full-width cut). Run after posting op1-top.nc."""
import sys
p = sys.argv[1]
s = open(p).read()
# op 1: before its wall rough; op 2: before its first lug pass (both straight after the slot rough)
mark = "(Rough the walls Z8.5 to Z5" if "(Rough the walls Z8.5 to Z5" in s else "(Lug pass 1/"
block = """(CHIP PAUSE: blow the chips out of the slots and their reliefs; delete down to END CHIP PAUSE to skip)
G53 G0 Z0
M5
M9
(MSG,Blow the chips out of the two slots and their end reliefs [spindle and coolant are off], then press Cycle Start)
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
