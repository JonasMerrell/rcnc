---
name: dmc2-feed-limits
description: Where the DMC2's 3000 mm/min feed cap comes from, rcnc axis limits, and the 5000 mm/min rapids/link_feed decision
metadata:
  type: project
---
- The 3000 mm/min "max feed" is only rCAD's job setting (cam.py M dict max_feed / guidelines), origin undocumented. It is not a mill limit.
- rcnc dmc2() config (crates/machine/src/config.rs, from Mach3 DMC2.xml): X/Y max_velocity 133.3 mm/s (8000 mm/min), Z 100 mm/s (6000), accel 2000 mm/s², 200 steps/mm. G0 runs at axis max_velocity × rapid override (no G0 feed). Mill PC's saved values unconfirmed.
- Jonas 2026-11-18: "lets run rapids at 5000mm/min". rCAD: machine.link_feed (air G1 moves at 5000, cutting stays max_feed 3000), branch link-feed stacked on feed-schedule PR 159; jobs set rapid_rate/rapid_z 5000. rcnc side is Jonas's setting (83.3 mm/s or ~62% rapid override). Jonas has already run 5000 mm/min air moves from Fusion on this mill, so no separate air test needed.
- Cutting max_feed above 3000 not yet decided (4000 would be 0.15 mm/tooth at 9k, 3 flutes).
- 2026-11-18: link_feed is rCAD PR 161 (branch link-feed, head 6bed0a3, stacked on PR 159). When re-planning any job, set cam.py machine link_feed 5000, rapid_rate 5000, rapid_z 5000. Gauge op1-top re-sim 1928 -> 1640 s, mostly from G0 at 5000.
