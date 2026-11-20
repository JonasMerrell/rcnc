"""A chip-clearing stop before the finishing that follows 9k roughing (Jonas: at 9k the chips aren't flung off):
spindle and coolant off, a message, M0, then the spindle back on with 3 s in air. Run after posting.

    python3 add_finish_pause.py program.nc "(Finish the slot walls" "Blow the chips out of the slots, the reliefs and off the top"
"""
import re
import sys
p, mark, what = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
assert mark in s, mark
i = s.index(mark)
assert "(END CHIP PAUSE)" not in s[max(0, i - 40):i], f"already a pause before {mark}"
# the speed the program runs at just before the mark (the pause restarts it)
spd = re.findall(r"^(S\d+) M3$", s[:i], re.M)
restart = f"{spd[-1]} M3" if spd else "S24000 M3"
block = f"""(CHIP PAUSE before the finish: delete down to END CHIP PAUSE to skip)
G53 G0 Z0
M5
M9
(MSG,{what} [spindle and coolant are off], then press Cycle Start)
M0
{restart}
M8
G4 P3
(END CHIP PAUSE)

"""
open(p, "w").write(s[:i] + block + s[i:])
print("added")
