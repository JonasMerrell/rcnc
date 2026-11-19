"""After every spindle start or speed change (S... M3), 3 s in air before anything else (G4 P3): the bog guard
learns the spindle's no-load speed at that speed there. Run after posting."""
import re
import sys
p = sys.argv[1]
lines = open(p).read().split("\n")
out = []
for i, l in enumerate(lines):
    out.append(l)
    if re.match(r"^S\d+ M3$", l.strip()) and not (i + 1 < len(lines) and lines[i + 1].strip() == "G4 P3"):
        out.append("G4 P3")
open(p, "w").write("\n".join(out))
print("dwells:", sum(1 for l in out if l.strip() == "G4 P3"))
