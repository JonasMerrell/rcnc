---
name: dmc2-find-vise-location
description: rcnc Find vise location wizard and FIND_STOCK VISE= (PR 21, 2026-11-19): what it measures, where it is saved, how CAM should use it
metadata:
  type: project
  modified: 2026-11-19T03:54:22.021Z
---
- rcnc PR 21 (MERGED 2026-11-19; not on the mill PC until Jonas says rebuild): Probe page > Vise > Find vise location. Vise empty, stops off; probe over centre block middle; touches top, both Y ends, both fixed jaw faces at +-30 Y. Saves probing.vise.location in machine.json (machine coords): faces [left station = block -X face, right station = +X face], middle y, angle, setter_above_top.
- FIND_STOCK VISE=-1 = fixed jaw on stock's -X side (right station); VISE=1 = left station. X from saved jaw (no touch, any stock length), checks stock face within TOL of jaw else Mismatch; "Over the stock" button. Not with JAWX/JAWZ; JAWAT ok.
- First real run failed ("Probe search finished without touching"): PR 21 touched the web as the top, faces below the ledges, missed the stepped-down ends. Fix (PR 22, MERGED 2026-11-20 as main 6248679; built on the mill PC 2eb5ac6 before merge): touch web, then +X bar top (start + gap/2 - 6.35), bar ends (jaw_length 95.5), faces face_depth 2 below bar top; errors if bar top < 2 above first touch. Settings renamed (jaw_length, face_depth, jaw_thickness) so old values are dropped.
- Arrow-key jog 'undone' (2026-11-19): no code lost; after Jonas restarted rcnc it jogged fine (likely an old copy still running).
- Defaults (PR 22) gap 33.27 / jaw length 95.5 from CAD: unverified on the real vise. Jaw angle reported, not applied as rotation.
- Related: [[dmc2-vise-geometry]], [[dmc2-op2-locating]], [[dmc2-touch-probe]]
