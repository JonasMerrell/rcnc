---
name: dmc2-toolpath-literature
description: 2026-11-18 literature review of toolpath optimisation vs rCAD; ranked follow-ups (feed scheduling first)
metadata:
  type: project
---
Report (Claude Doc): https://claude.ai/artifact/KGDcNz6wV7P9TETBeUEHaS "rCAD toolpaths vs the literature" (2026-11-18).
Ranked: 1) per-move feed scheduling from the sim's measured load (adaptive runs ~260-300 W vs strips ~430 W because AREA_SHARE 0.4 / POWER_SHARE 0.7 protect wall spikes); 2) measure engagement angle in Level::grow so the share can rise; 3) calibrate chatter (Altintas-Budak already coded) with a tap test on the spindle transducer; 4) measure predicted vs cut-log time before building smooth spiral paths (cycle_time ignores acceleration); 5) 2-opt ordering (low); 6) trochoidal slots.
Item 1 BUILT 2026-11-18: rCAD PR 159 (feed-schedule, on power-target PR 158), src/cam_feed_schedule.rs, opt-in feed_schedule flag; limit_feeds uses it. Result: small on real jobs (gauge op1 adaptive already at 3000 mm/min ceiling at 9k, unchanged; slider programs 0-10% shorter); grid reads up to ~10% over target. Next lever on DMC2 is wider stepover (item 2), not feed. Item 1 should follow the climb-loops / stay_down / power-budget PRs (155-157) from [[dmc2-cutting-simulation]] work.
