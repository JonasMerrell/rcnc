# Ingot slider: how to run it

Two 6061 bases per 1/2 × 1 in bar blank, two C360 brass sliders per 3/8 × 1 in bar blank, two vise setups each,
plus a separate engraving program for the sliders. Part sizes, design changes and the jaw/shim arithmetic are in
`DESIGN.md`; per-operation tools, feeds, times and loads are in `OPS.md`.

Programs were made with rCAD branch `cam-slider-fixes` (PR #147); they need no rcnc changes (all four with probing
ran to the end in the rcnc simulator on `mill/2026-11-14`).

| Program | Station | Time | Sheet |
|---|---|---|---|
| `base-op1.nc` | left (raw bar) | 5.3 min | `base-op1.html` |
| `base-op2.nc` | right (flipped) | 14.2 min | `base-op2.html` |
| `slider-op1.nc` | left (raw bar on the 1 mm shim) | 11.9 min | `slider-op1.html` |
| `slider-op2.nc` | right (flipped) | 6.6 min | `slider-op2.html` |
| `slider-engrave.nc` | right, straight after `slider-op2` | 2.5 min | `slider-engrave.html` |

Copy each `.html` beside its `.nc` (rcnc shows `<program>.html` as the setup sheet).

## Blanks and tools

- Saw blanks **99.5 ± 0.5 mm** long (op 1 squares them to 98.0; it accepts 98.5 to 101, TOL 1.5).
- Aluminium: 6061 1/2 × 1 flat bar (12.7 × 25.4). Brass: C360 3/8 × 1 (9.5 × 25.4).
- **Shim for brass op 1:** a flat 1.0 mm plate about 25 × 90 (any steel or aluminium; shorter than the 95.5
  jaws, so the end trims at ±49 never reach it). Op 1 brass only.
- T24 3 FLUTE DLC 1/4 (stick-out 24), T1 ALUMINUM chamfer 90 (stick-out 22), T99 probe. Flood coolant.
- Speeds: aluminium 22 500 rpm / 2350 mm/min (24k flagged chatter in rCAD's stability model on op 1's rough;
  22.5k is its best speed), brass 18 000 rpm / 1350 mm/min roughing and finishing, chamfer 1000.

## Stations

The vise's centre block is the fixed jaw of both stations. Programs assume (`on_table`):
- **Op 1 in the left station**: the fixed jaw is on the part's **+X (right)** side. Op 1 takes X and Y from the
  raw bar with Find stock (both sides), so which jaw is fixed doesn't matter for it.
- **Op 2 in the right station**: the fixed jaw on the part's **−X (left)** side, as in the earlier jobs. Op 2 takes
  Y from the two finished ends and X from the middle of the raw band's two sides: the same bar faces op 1
  centred on. X can't come from the fixed jaw: rcnc touches the jaw beyond the stock's end, and the 98 mm blank
  is longer than the 95.5 jaws.

Close the left station's moving jaw down to 25.4 (it is set at 40 in the CAD).

## Op 1 (bottom side up), left station

Bar on the ledges, centred along the jaws (both ends stick out about 2 mm past them), clamped on the bottom
4.7 mm. **Brass: the 1 mm shim under it, centred.**

1. Open `base-op1.nc` / `slider-op1.nc`, Cycle Start.
2. T99 + Find stock: jog over the middle, Continue. Aluminium keeps Z from the Vise reference; brass sets Z from
   the bar top (Z0 = the bar's bottom on the shim).
3. T24: face the top (0.5 / 0.2), rough round both parts and the 8 mm gap, one full-height finish pass on the
   walls (down to Z6.5 alu / Z4 brass: never lower than 0.3 over the jaw tops), brass: the 6 magnet pockets;
   square both ends to 98.0 full height (beyond the jaws, they're the parts' outer ends and op 2's Y datum).
4. T1: 1.5 chamfer on the bases' bottom outline / 0.3 edge break on the sliders' bottom outline and pocket rims.

## Op 2 (top side up), right station

Turn the blank over (either way: both walls and both ends were finished in the same op 1 passes): finished bottom down on the ledges, op 1's finished walls in the jaws (they reach 5.7 alu / 5.3 brass
above the ledge, the jaws grip 4.7). Push the finished wall against the fixed jaw, clamp. The raw band above the
walls is wider (25.4): make sure the jaws close on the finished walls, not on it.

1. Open `base-op2.nc` / `slider-op2.nc`, Cycle Start.
2. T99 + Find stock: Y from the two finished ends (98.0 ± 1), X from the raw band's sides (25.4), Z from the
   Vise reference.
3. Base: face to 9, rough channel and outside to Z6 (the two bases still joined by a 0.3 web in the gap), finish
   the channel floor and the rails, magnet pockets, then the rest rough below Z6 that **separates the two bases**
   (light: 0.3 deep), the outline's full-height finish pass (9 to 5), T1: 1.5 rail-top chamfers, 0.3 edge breaks.
4. Slider: face to 8, rough the sides and gap to Z5 (**separates the two sliders** during this rough), one
   full-height finish pass, T1: 2 mm ingot chamfer.
5. Sliders: without unclamping, open `slider-engrave.nc` and run it (T1 again, same work offset, no probing).
   Swap this program for a custom pattern or name: any sketch on the top field (15.6 × 41 flat, keep 1 mm in
   from the chamfer) and a `trace` (lines) or `vcarve` (text) op in the "Slider op2 engrave" tab of `slider-cam.json`.

**Separation risk:** after the separating cut each part is held only by its own 45 mm of 4.7 mm grip; the cuts after
it are light (≤ 0.3 mm stock, ≤ 8% spindle). Both parts of a blank were finished in the same op 1 pass, so they
are the same width and the jaws hold both. Check the jaw is really tight on both before the separating op; don't
skip the chip blow-off between parts.

## Magnets

N52 Ø8 × 3, glued (e.g. Loctite 638 / slow epoxy) 0.15 below flush. Mark polarity before gluing: for a detent,
each base magnet faces the slider with the opposite pole of the slider magnet above it (all base N up, all slider
S down). Try a polarity layout dry first (see the open questions).

## Checked

rCAD stock + machine simulation of all five programs: no problems (crashes, jaw hits, gouges, unfinished faces).
Peak loads: base op 1 rough 64% of the spindle, everything else ≤ 48%. The DMC2 machine model with the vise jaws
is in every setup, and the jaws are fixtures that the toolpaths keep 0.3 over. `base-op1`, `base-op2`, `slider-op1` and `slider-op2` ran to the end in the rcnc simulator (Find stock passes;
7.3, 14.8, 13.9 and 9.0 min simulated with tool changes). `slider-engrave` needs op 2's work offset, so it wasn't run alone.
