# Ingot slider: slider programs

Everything for the slider: programs (.nc), setup sheets (.html), pictures and the STEP. How to set up and run
them is in [../RUN.md](../RUN.md).

Batch order:
1. `slider-first.nc` on the first raw blank (op 1 only, left station, G54).
2. `slider-cycle.nc` once for every blank after it (op 1 left + op 2 right in one program).
3. `slider-last.nc` for the last blank (op 2 only, right station, G55; with the engraving and the 2-pass chamfer).

Op 1 lies on the two 1.67 mm shim strips.

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
