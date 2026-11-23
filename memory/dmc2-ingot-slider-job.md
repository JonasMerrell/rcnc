---
name: dmc2-ingot-slider-job
description: Ingot fidget slider gift job: dimensions, stock, workholding, programs on the mill PC, first-run feedback (slider cut 2026-11-18), branches/PRs, open points
metadata:
  type: project
  modified: 2026-11-20T04:37:07.001Z
---
State 2026-11-20 5 PM MT: slider cut once (clean); base-first cut (8.6 version); programs without aluminum chip pauses on the mill PC.
- Files: netdash ~/dmc2/slider (branch slider-job; `./replan.sh slider|base` ~4 min); rcnc build branch mill/ingot-slider-vise (5080cf5 = rcnc 2a7bb0d code + jobs/ingot-slider/{slider,base}/); mill/ingot-slider 9d3f2df same files on older main. Project files ingot-slider/. No HTML playbacks.
- Base 6061 25 x 45 x 8.6 (was 9.0; Jonas's idea 2026-11-19 so the seam hides with no shim), channel 20 x 3, R2.5, rail chamfer 1.7, 0.3 bottom break (must seat on 1.9 ledges), 2.45 under magnets. Slider C360 19.6 x 45 x 8, R3, ingot chamfer 2.4, engraving 0.4. Magnet pockets 8.05 x 3.15 at y -15/0/+15 (8x3 N35: perfect press fit, Jonas).
- Stock measured: brass 10.0 x 25.0, aluminum 12.34 x 26.6, blanks 101. Tools T99, T24, T1.
- Workholding: brass op 1 on 1.67 mm shims (grip 3.0, rough capped 400 W); aluminum no shim (grip 4.7). Op 1 left G54, op 2 right G55. Op 2 = Find stock (FIND_STOCK VISE=1 op 1 / -1 op 2, needs one-time Find vise location), blow-out pause, Find bores 1 mm below carrier top. Seam hidden 0.24 (base) / 0.87 (slider) inside the top chamfers.
- Op 2 order: face, light band rough, engrave, 2-pass chamfer (no extra tool change: Jonas rejected separating last). Op 1 wall finish one full-height pass.
- Speeds: aluminum rough 9k (~680-690 W of 850), face/finish 12k, chamfer 24k; brass 18k. Bog guard required.
- Programs per folder: -first (op 1), -cycle (both stations), -last (op 2; slider-last includes engraving). Batch: first, cycle per blank, last. Cycles base 13.6 / slider 13.8 min.
- Mill PC: rcnc 2a7bb0d, jobs\ingot-slider\{slider,base,old}; slider = 52ad3df programs, base = 5080cf5. Find vise location has been run (base run used it).
- Run 1 logs (rcnc logs/ingot-slider-run-1): brass droop mean 1-2.6%, max ~4%, one 11% spike in magnet helix; Jonas: cycle time fine, most time is tool swaps, do not speed up. Slider Y shift in run 1 likely an under-clamped part.
- Base run 1 (base-first 8.6 version, 2026-11-20 ~5 PM MT; logs/ingot-base-run-1 2c2b243): 9k rough droop mean 3.7%, max 8%; others ~3%. Jonas: no chip pauses needed in aluminum (removed, 5080cf5 on the mill PC 2026-11-20).
- rCAD PRs open for Jonas: #163, #165 (build cache), #166 (shared sim, Find bores/contour fixes, verify fix).
- Open: brass feed schedule up to 2850 on light cuts; protected jaws for op 2; chatter model uncalibrated; magnet polarity.
- Eclipse 1G variants (engraved + 3D programmed 2026-11-21, not on the mill PC; true-proportion custom-base version proposed): see [[dmc2-eclipse-slider]].
- 2026-11-22 Jonas: brass slider magnet fit great on the first 2 sliders, too tight on the rest; bases perfect throughout. Pocket toolpath identical (single spiral, no finish/spring lap, only ~220 deg at final depth); cause: cutter re-seated every tool change, runout varies (Jonas). Jonas chose: brass pockets 8.07 + full lap + spring lap, glue if loose; bases stay 8.05 Also a bug: old bottom lap left a 0.17 wedge on 1/3 of the floor edge (fixed, rCAD PR 175 unmerged; bases still have it, fit fine). Posted mill/ingot-slider-vise fca5cd0, NOT on mill PC: slider first c761fc5b4397, cycle 3dbdf5ac75c1, last f28d459fc8a0.
- Lessons: judge cuts in model watts; gaps between parts set the peak; confirm stock, shims, blank length first.
- Related: [[dmc2-gift-ideas]], [[dmc2-vise-geometry]], [[dmc2-cam-preferences]], [[dmc2-cutting-simulation]], [[dmc2-find-vise-location]]
