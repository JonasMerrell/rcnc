"""op1a v2's program marks, after posting: the stock it's for, the bog guard required, the recording, and before
each pass a line rcnc shows ("Pass N: ...") and 3 s of the spindle in air (the wall meter's reference, and the
bog guard re-learns the no-load speed there after an S change)."""
import re
import sys

p = sys.argv[1]
s = open(p).read()
lines = s.split("\n")
out = []
first = True
i = 0
while i < len(lines):
    l = lines[i]
    out.append(l)
    if i == 0:
        out.append("(STOCK 32 x 32 x 120.7 - 6061 bar as measured 2026-11-17; NOT for a 25.4 bar)")
    if l.strip() == "(RCNC Z_REFERENCE Vise)":
        out.append("(RCNC REQUIRE_GUARD: the bog guard [Settings -> Listening, feed hold on] must be armed: rcnc refuses Cycle Start without it)")
        out.append("(RCNC RECORD_AUDIO: the calibration passes; rcnc saves the run's sound beside this file as .wav)")
    m = re.match(r"\((C\d+) (.*) - Trace\)$", l.strip())
    if m:
        # the tool change and an S change come right after the comment: keep them before the dwell
        j = i + 1
        while j < len(lines) and re.match(r"^(G53 G0 Z0|T\d+ M6|S\d+ M3|M8)$", lines[j].strip()):
            out.append(lines[j])
            j += 1
        label = m.group(2).replace("(", "[").replace(")", "]")
        out.append(f"(MSG,Pass {m.group(1)[1:]}: {label} - spindle in air 3 s, then the cut)")
        out.append("G4 P3")
        i = j
        continue
    i += 1
open(p, "w").write("\n".join(out))
print("marked")
