---
name: iso20-atc-spindle-model
description: Where the real ISO20 ATC spindle model lives in ~/dmc2 and its open fit issues (as of 2026-11-14)
metadata:
  type: project
  modified: 2026-11-14T21:31:01.778Z
---

The real ISO20 ATC spindle (94.5 x 81.8 body, 372.7 long, Ø67 nose) was modelled on 2026-11-14 on netdash.

- Generator: `scripts/spindle.py` from `atc/spindle.json` (numbers sorted into printed / scaled / invented). Output: `parts/spindle/iso20-atc-spindle.json` and `.step`; assumptions in `parts/spindle/README.md`.
- `scripts/assembly3.py` imports it in place of the old blue envelope; nose tip and axis unchanged.
- Open: mounting face (opposite the power connector) is 40.9 from the axis but the Z plate is 55 from it, so a 14.1 mm adapter plate is needed and not yet modelled. `design.json` `iso20_body` still has the old box with width/depth swapped.
- Drawing callout says 6x M6 mounting holes but shows 7; all 7 modelled pending Jonas measuring the real spindle.

**Why:** later threads need to know the stand-in is gone and what is still unresolved.
**How to apply:** change spindle dimensions in `atc/spindle.json`, not in generated files.

2026-11-15 second option: G-Penny GDL80-20-24Z/2.2 (round Ø80 x 346 body, 350.7 overall, 10 kg, water cooled, all lines out the rear, built-in NPN tool locked/released sensors). Modelled in `parts/spindle-gdl80/` from `atc/spindle-gdl80.json` via `scripts/spindle_gdl80.py`; assembly built in `parts/asm3-gdl80/`; select with `assembly3.py` fifth argument `spindle-gdl80` (default still the square spindle). Fits the existing Ø80 clamp, no adapter plate, zero interference at the six tool-change poses. Open: "ISO20-15°" probably means a 15° pull stud vs the Damen holder's 45° stud; air pressures, water flow and 110 V winding current unconfirmed with vendor. CAM spindle numbers live in `stand/stand-cam.json`, `job/cam.json`, `job2/cam.json`, rcnc `spindle_max_rpm`.
