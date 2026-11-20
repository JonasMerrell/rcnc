"""Every program: rcnc's bog guard must be armed (it holds the feed when the heard spindle speed sags; rcnc
refuses Cycle Start without it). The cuts are planned under the spindle's measured pull-out (guidelines.md)."""
import sys
for p in sys.argv[1:]:
    lines = open(p).read().split("\n")
    assert not any("REQUIRE_GUARD" in l for l in lines), p
    lines.insert(1, "(RCNC REQUIRE_GUARD: the bog guard [Settings -> Listening, feed hold on] must be armed: rcnc refuses Cycle Start without it)")
    open(p, "w").write("\n".join(lines))
    print("marked", p)
