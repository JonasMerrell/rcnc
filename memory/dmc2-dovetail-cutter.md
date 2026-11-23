---
name: dmc2-dovetail-cutter
description: Jonas's Ø4 45° dovetail cutter and rCAD dovetail support (PR 170): back chamfers and dovetails via Contour
metadata:
  type: project
  modified: 2026-11-21T20:39:13.389Z
---
- Tool (Jonas 2026-11-21): Ø4 × 45° dovetail, Ø2 neck 6 mm long, Ø4 shank, 50 mm OAL, 4 flutes, uncoated carbide. Row in project files tools/dmc2-tools.csv. Flank height assumed 1 mm (45° from the bottom face); asked Jonas to confirm by measuring (2.4 mm would mean included angle). Unconfirmed as of 2026-11-21.
- Jonas wants it for BOTH back-deburring undersides (through holes, part bottom edges) and dovetail grooves/slides.
- rCAD PR 170 (MERGED 2026-11-22; not on the mill PC until rebuilt): ToolKind::Dovetail (angle = flank from bottom face, neck_length), CSV import, sim cuts undercuts (per-cell gaps), gouge check against part solid, shank-above-neck check, neck in deflection model. Contour with dovetail: chamfer>0 = back chamfer under edge at bottom (tip below edge, chamfer mid-flank); chamfer 0 = dovetail slot_depth into wall.
- Reach limit: the Ø4 shank starts 6 mm above the tip, so back chamfers only on edges up to ~5.5 mm below the wall top beside the cut; the sim flags shank hits.
- Related: [[dmc2-cam-preferences]] (every edge deburred).
