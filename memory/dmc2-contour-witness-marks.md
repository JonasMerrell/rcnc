---
name: dmc2-contour-witness-marks
description: Jonas saw witness marks where finishing contours start/stop; the lead-in/overlap defaults being added to rCAD
metadata:
  type: feedback
---
Finish contour passes must not leave a start/stop witness mark (Jonas, 2026-11-15, after the phone stand: rCAD already used quarter-arc leads mid-side; the mark came from no overlap and one full-depth lap).
**Why:** visible mark on finished walls of both real parts.
**How to apply:** finish contours get 1-2 mm overlap past the start, a light final lap (0.02-0.05 mm), lift along the lead-out, and a choosable seam location. Being built in rCAD on a new branch off rcnc-find-stock (thread "Make real parts", 2026-11-15), rCAD PR 146 (branch contour-leads, stacked on PR 140), merged into rcnc-find-stock 2026-11-15 on Jonas's "Merge" (91674b8); PR 140 to master still open; untested on the mill. Related: [[dmc2-cam-preferences]]
