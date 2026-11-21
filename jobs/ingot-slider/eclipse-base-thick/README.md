# Ingot slider: thick-rail base for the Eclipse car (6061)

The normal ingot base (`../base`) with one change: **the channel is 17.7 wide instead of 20.0**, so each rail is
**3.65 wide** (was 2.5). Everything else as the ingot base: 25 × 45 × 8.6, R2.5 corners, channel 3.0 deep, rail-top
chamfer 1.7 outside, 0.3 edge breaks (bottom outline, channel edges, floor ends, pocket rims), three magnet
pockets 8.05 × 3.15 at −15 / 0 / +15. The car (17.3 wide) has 0.2 each side in the channel.
Kept in its own folder so it can't be mixed up with the 20.0 base. CAD: `eclipse-base-thick.step`, `base-iso.png`,
`base-front.png`; on it, the car: `../eclipse-car/asm-iso.png`.

Same bar (measured 12.34 × 26.6, blanks 101), same vise stations, CAM and checks as the ingot base: op 1 on the
vise ledges with Z from the Vise reference (`KEEPZ=1`), op 2 also from the Vise reference, Find stock from the
saved vise location (op 2 `JAWAT=-12.5`), blow-out pause, Find bores, bog guard required, dwells.

| Program | What | rCAD time | rcnc simulator | sha256 |
|---|---|---|---|---|
| `eclipse-base-thick-first.nc` | op 1 alone (G54) | 2:36 | 359 s, ran to the end | `a6462995895f` |
| `eclipse-base-thick-cycle.nc` | op 1 + op 2 (G54 + G55) | 7:11 | 750 s, ran to the end | `30545bb26071` |
| `eclipse-base-thick-last.nc` | op 2 alone (G55) | 4:35 | 529 s, ran to the end | `0199c1672afd` |
| `eclipse-base-thick-op1.nc`, `-op2.nc` | single ops | 2:36 / 4:56 | 359 / 529 s | `a6462995895f` / `233cc028db87` |

All SIM OK (0 problems); `verify_gcode` on the final files: **all 5 ok, 0 problems, stamps match the setups**.

Simulation: op 1 peak 679 W at 9k (rough, caution 850), chatter "expected" on the rough (1.54) and caution on the
wall finish (0.74); op 2 peak 675 W at 9k (channel rough), chip caution in the channel rough (0.57), as the
normal base's numbers that have run; nothing unfinished. Op 2's "rough the band" finds nothing to clear (the
face takes it), as on the normal base.
