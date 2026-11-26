# Ingot slider: thick-rail base for the Eclipse car (6061)

The normal ingot base (`../base`) with one change: **the channel is 17.7 wide instead of 20.0**, so each rail is
**3.65 wide** (was 2.5). Everything else as the ingot base: 25 × 45 × 8.6, R2.5 corners, channel 3.0 deep, rail-top
chamfer 1.7 outside (in 6 passes, 2026-11-26), 0.3 edge breaks (bottom outline, channel edges, floor ends, pocket rims), three magnet
pockets 8.05 × 3.15 at −15 / 0 / +15. The car (17.3 wide) has 0.2 each side in the channel.
Kept in its own folder so it can't be mixed up with the 20.0 base. CAD: `eclipse-base-thick.step`, `base-iso.png`,
`base-front.png`; on it, the car: `../eclipse-car/asm-iso.png`.

Same bar (measured 12.34 × 26.6, blanks 101), same vise stations, CAM and checks as the ingot base: op 1 on the
vise ledges with Z from the Vise reference (`KEEPZ=1`), op 2 also from the Vise reference, Find stock from the
saved vise location (op 2 `JAWAT=-12.5`), blow-out pause, Find bores, bog guard required, dwells.

| Program | What | rCAD time | rcnc simulator | sha256 |
|---|---|---|---|---|
| `eclipse-base-thick-first.nc` | op 1 alone (G54) | | 359 s, ran to the end | `4682b7107c70` |
| `eclipse-base-thick-cycle.nc` | op 1 + op 2 (G54 + G55) | | 807 s, ran to the end | `f039895216bc` |
| `eclipse-base-thick-last.nc` | op 2 alone (G55) | | 586 s, ran to the end | `6cbafac3c0c4` |
| `eclipse-base-thick-op1.nc`, `-op2.nc` | single ops | | 359 / 586 s | `4682b7107c70` / `2e9930306646` |

All SIM OK (0 problems, at the fine setting: 1000 cells); `verify_gcode` on the final files: **all 5 ok, 0 problems, stamps match the setups**. Re-posted 2026-11-22 with rCAD PR #175: same toolpaths, but the T1 deburr passes are now written as short lines instead of arcs (the post fits arcs to 3D finishing and deburr passes only where the points already lie on them).

Simulation: op 1 peak 679 W at 9k (rough, caution 850), chatter "expected" on the rough (1.54) and caution on the
wall finish (0.74); op 2 peak 675 W at 9k (channel rough), chip caution in the channel rough (0.57), as the
normal base's numbers that have run; nothing unfinished. Op 2's "rough the band" finds nothing to clear (the
face takes it), as on the normal base.

## 2026-11-26: rail chamfer in 6 passes

Jonas: the 1.7 rail-top chamfer came out poorly. It was one deburr pass that also took the 0.3 left on the band,
about 1.4 mm deep square to the chamfer face. Rule now (guidelines): at most 0.3 per pass, the last pass at size 0.1.
The chamfer is cut 0.072 / 0.443 / 0.815 / 1.187 / 1.559 / 1.7 wide (about 0.26 deep each, last 0.1), tip 1.0 under
each pass's lower edge (`chamfer_passes()` in `../cam.py`). Re-posted with the reference rCAD build 063b2ff; SIM OK,
`verify_gcode` ok (0 problems) on all five files, rcnc simulator (2a7bb0d + local vise harness) ran all three to the
end. Op 1 also differs from the earlier post in the carrier bores' exit move (the 063b2ff bore lap fix; the earlier
base programs were posted before it). The car programs re-posted in the same run came out byte-identical.
rCAD warns on T1 at 24k (479 m/min at the 6.35 diameter), as before; the cone cuts at a smaller diameter.
