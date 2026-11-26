# Ingot slider: slider programs

Everything for the slider: programs (.nc), setup sheets (.html), pictures and the STEP. How to set up and run
them is in [../RUN.md](../RUN.md).

Batch order:
1. `slider-first.nc` on the first raw blank (op 1 only, left station, G54).
2. `slider-cycle.nc` once for every blank after it (op 1 left + op 2 right in one program).
3. `slider-last.nc` for the last blank (op 2 only, right station, G55; with the engraving and the 2-pass chamfer).

Op 1 sits on the vise ledges with **no shims** since 2026-11-26 (below). Before that it lay on two 1.67 mm shim strips.

## 2026-11-26: op 1 without shims on the cut left jaws (it deepens the jaw cut)

The first Eclipse car op 1 ran without its shims and cut 1.37 off the left-station jaw tops: they now grip
**3.33** at their faces. With the 1.67 shims a slider bar would be held by only ~1.7 mm there, so op 1 now runs
**without shims**, the bar on the ledges, Z from the vise (`(RCNC Z_REFERENCE Vise)`; no REQUIRE_SHIM). T24 goes
down to **Z3.63** beside the jaws (was 3.33 on the shims). Where the slider's path reaches past the car's cut,
it **cuts the jaw bars up to 1.07 deeper, to 3.63 above the ledges, out to ~7 mm from their faces**. Jonas OK'd
that ("let it deepen"), and each program's header says so (`NOTE: slider op 1, NO SHIMS`). Op 1's walls now reach
6.17 up the part (was 6.47). The 2.4 ingot chamfer's lower edge stays at 5.6, so the seam is still 0.57 inside
the chamfer. The ingot chamfer is now **8 passes** (at most 0.3 deep each, last 0.1).

The rCAD vise models carry the deeper cut, so the SIM OK gate passes against the jaws as they will be after
this run. The gate copy is grown 0.8 mm round the computed cut for rCAD's 1 mm collision grid. The reference
CAD is `vise-2026-11-26-left-jaws-cut` in DMC2-cad.

| Program | rCAD time | rcnc simulator | sha256 |
|---|---|---|---|
| `slider-first.nc` / `slider-op1.nc` | 5:12 | 496 s, ran to the end | `a8a495128552` |
| `slider-cycle.nc` | 11:32 | 955 s, ran to the end | `0efee661ee3a` |
| `slider-last.nc` | 11:32 (job) | 603 s, ran to the end | `8753a3ac70a6` |

All have SIM OK and pass `verify_gcode` with 0 problems. Posted with rCAD 063b2ff.

## 2026-11-22: magnet pockets 8.07, round to the floor

The 8.05 pockets came out tight after the first blank (Jonas: the cutter is re-seated in the collet at every
tool change, so runout and hole size vary). Op 1 (`slider-first`, `slider-cycle`, `slider-op1`) now cuts them
**8.07** × 3.15: helix 0.1 inside the wall, then a full lap at size at full depth and a spring lap (rCAD PR #175:
the old bottom lap restarted at 0° and left a wedge up to 0.17 at the floor edge). Depth and the 0.3 rim break
unchanged; a loose magnet gets a drop of glue. The aluminium base pockets stay 8.05 (not re-posted).

| Program | rCAD time | rcnc simulator | sha256 |
|---|---|---|---|
| `slider-first.nc` / `slider-op1.nc` | 4:57 | 491 s, ran to the end | `c761fc5b4397` |
| `slider-cycle.nc` | 9:16 | 835 s, ran to the end | `3dbdf5ac75c1` |
| `slider-last.nc` | 4:18 | 488 s, ran to the end | `f28d459fc8a0` (op 2 moves unchanged; new stamp) |

All SIM OK at the fine setting (1000 cells), `verify_gcode` ok.
