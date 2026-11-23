---
name: dmc2-vise-setup-preferences
description: Jonas's rules for vise setups on the DMC2 (Z0 at vise floor, fixed jaw left, grip height, nothing above the jaws, no new holes)
metadata:
  type: feedback
---
- Z reference for vise jobs (Jonas, 2026-11-14): work Z0 = bottom of the vise (vise floor) via rcnc vise reference, parallel height in the setup; do not take Z from the raw stock top. First part came out 16.13 thick (target 16) because Z was taken from raw faces.
- Vise setup: fixed jaw on the LEFT (-X side) (Jonas, 2026-11-14); jaws clamp along X. Fixed-jaw probing gives X.
- Vise jaws grip about 4.7 mm above the built-in parallels (Jonas confirmed 2026-11-14).
- Fixtures/stops on the vise must not stand above the jaw top (Jonas rejected a 9 mm plate on the fixed jaw, 2026-11-14: gets in the way of the tool/holder).
- No new holes in the vise (Jonas, 2026-11-14): vise accessories must use existing holes/features or clamp on.
- Related: [[dmc2-cam-preferences]], [[dmc2-machine-facts]]
