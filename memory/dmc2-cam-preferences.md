---
name: dmc2-cam-preferences
description: Jonas's CAM preferences learned from real cuts on the DMC2 (chamfer tip offset etc.)
metadata:
  type: feedback
  modified: 2026-11-15T21:07:58.584Z
---
- Chamfers: use a tip offset so the chamfer mill cuts farther down the cone (about 1 mm below the chamfer's lower edge), not near the point.
  **Why:** first real part (op 1, 2026-11-14) had a poor chamfer finish cutting near the tip.
  **How to apply:** default ~1 mm tip offset; check clearance in the sim. NOT on edges next to a face shallower than the 45 deg cone: stick-out gauge op 1 (2026-11-17) gouged all round the thumb dish rim (Jonas confirmed); use the ball edge break there.
- Contours: rough in step-downs leaving ~0.2 mm radial, then one full-height finishing pass.
- Never rough outside walls with a single contour pass when raw stock is square and the part has rounded corners: phone stand op 1 (2026-11-15) slotted ~5.6 mm at each R10 corner and overloaded T24. Rough walls with adaptive (bounded engagement), then the full-height finish pass.
- rCAD stock sim has a tool-load check (PR 140), calibrated on real DMC2 cuts. Always check per-op peak load before sending a job to the mill.
- Every surface always gets a finishing pass (Jonas, 2026-11-15). Flat floors: finish with a flat end mill; audit all surfaces before showing a job.
- Every edge always gets deburred (Jonas, 2026-11-15): chamfer mill where it reaches, otherwise a tiny edge break (~0.2 mm) with the ball end mill. Audit all edges, like surfaces, before showing a job.
- Roughing style: until 2026-11-17 the rule was deep and narrow at 24k (weak spindle; 1.8 x 6 broke T24). SUPERSEDED by the VFD fix (see MEMORY.md spindle line): rough at S9000, default ~650 W, caution 850 W. Jonas 2026-11-17 on the first 9k job (op1-top, 11-13 mm deep strips): "chip buildup is much worse with the large depth of cut"; on top of the bar chips "don't really get flung at all" at 9k. So at 9k use moderate depth (about 5-7 mm) with wider step over, thick short chips, a chip-clearing pause before any finish pass that follows 9k roughing, and 12k where load allows.
- Ball mill finishing (Jonas, 2026-11-15): ball always at 24k rpm, 3000 mm/min. Default for 3D finishing. Stepover must be smaller than the phone stand's (0.48 mm parallel up to 65 deg slope, 0.45 mm waterline steps on the 6 mm ball: Jonas found the cusps too large). New default (mine, uncut): cusp <= ~0.005 mm measured along the surface, i.e. ~0.3 mm parallel limited to slopes < ~45 deg, ~0.2 mm waterline steps.
- For fixes while Jonas waits at the machine: quick .nc edit and push first, proper re-post/sims after; tell him the moment the file is on the mill PC. Restart programs must not re-probe partly cut stock (rcnc keeps G54 through Stop/Open/restart).
- Air time (Jonas 2026-11-17; op1-top lost ~10 of 40 min to feed plunges in air): rapid down to 1-2 mm above remaining stock, feed only into material; retract only over stock actually left; stay down and go around the part when faster.
- Direction and pass shape (Jonas 2026-11-18): adaptive and contours climb, finishing always climb; passes go all the way round the part as full laps; two-way only for open-slab roughing where it saves time (Jonas agreed 2026-11-18); size cuts from the spindle power budget, not fixed stepovers; handle corners properly (roll round outside corners, bounded engagement in corners).
- Op 2 locating rules: [[dmc2-op2-locating]].
- Vise setup rules: [[dmc2-vise-setup-preferences]].
- Wall finishing (Jonas 2026-11-18, ingot slider run): finish contours in ONE full-height pass, never split into depths (cleaner); chamfers with a big cut get a roughing pass plus a finishing pass.
- Chip pauses (Jonas 2026-11-20, after cutting the ingot bases): no chip-clearing M0 before finishing in the aluminum base runs, chips did not build up (shallow 9k roughing). Keep the blow-out pause before Find bores. My reading: still consider a chip pause for deep pockets/grooves.
