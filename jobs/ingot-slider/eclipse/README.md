# Ingot slider: Eclipse car (v3) — rides the custom base in ../eclipse-base

> **SUPERSEDED (2026-11-21): do not run these programs.** Jonas approved a new car surface and the thick-rail base; use `../eclipse-car/` (car) and `../eclipse-base-thick/` (base) instead. Kept for reference only.

A brass slider shaped like a 1st-gen Eclipse: side profile traced from Jonas's blueprint (eclipse-blueprint.png;
heights at 0.8 of true scale, 9.5 at the roof), body line at 3.7 (vertical lower body, 45° shoulder, near-vertical
sides, narrower greenhouse), wing-height tail, pop-up lids and glass outlines engraved. Underneath: a 10.4 × 1.65
slot that rides the base's centre rail, with the 3 magnets in its ceiling. The whole car stands on the base's road.

Batch order (same vise, carrier bores, Find stock from the vise location, bores pause, bog guard as the ingot):
1. `eclipse-first.nc` on the first raw blank (op 1 only, left station, G54, **no shims**, Z from the vise).
2. `eclipse-cycle.nc` once for every blank after it (T99, T24, T20, T1: 4 tool changes).
3. `eclipse-last.nc` for the last blank (op 2 only, right station, G55).

**Op 2 stands on the two 1.67 mm shim strips** (25 × 39, 12 apart, under each car, clear of the carrier bores):
that lets the tool go 1.67 lower beside the jaws, so the body can start curving at 3.7. Z still comes from the
vise (the programs' work zero is the ledges; the shims are in the model). Op 2 grips 3.0 mm of the car.

T20 = the 2 mm tapered ball, ASSUMED carbide, R1 tip, 7.5° per side, 1/8 in shank, 12 mm flutes, 18 mm out,
24 000 rpm F1000: tell me the real numbers and I'll re-post.
