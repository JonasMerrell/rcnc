"""Op 1 / op 2 marks after posting: the stock the job is for and the bog guard required (rcnc refuses Cycle Start
unless Settings -> Listening has the feed hold on and armed)."""
import sys
p = sys.argv[1]
s = open(p).read()
lines = s.split("\n")
assert "REQUIRE_GUARD" not in s
lines.insert(1, "(STOCK 32 x 32 x 120.7 - 6061 bar as measured 2026-11-17, after op1a v2; NOT for a 25.4 bar)")
lines.insert(2, "(RCNC REQUIRE_GUARD: the bog guard [Settings -> Listening, feed hold on] must be armed: rcnc refuses Cycle Start without it)")
open(p, "w").write("\n".join(lines))
print("marked")
