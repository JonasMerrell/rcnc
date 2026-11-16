# Ingot slider: how to run it

Two 6061 bases per 1/2 × 1 in bar blank and two C360 brass sliders per 3/8 × 1 in bar blank. Each part gets two
vise setups, and the sliders get a separate engraving program. Part sizes, design changes and the jaw/shim
arithmetic are in `DESIGN.md`. Per-operation tools, feeds, times and loads are in `OPS.md`. The job follows
`jobs/guidelines.md`; where it differs, it says so below.

Programs were made with rCAD branch `cam-slider-fixes` (PR #147). The single programs need no rcnc changes beyond `mill/2026-11-14`. The cycle programs need rcnc PR #14 (see below).
Op 1 runs in G54, op 2 and the engraving in G55 (rCAD `work_offset`).

| Program | Station | Time | Peak spindle | Sheet |
|---|---|---|---|---|
| `base-op1.nc` | left | 5.3 min | 60% | `base-op1.html` |
| `base-op2.nc` | right | 12.9 min | 40% | `base-op2.html` |
| `slider-op1.nc` | left (bar on two 1 mm shims) | 12.0 min | 36% | `slider-op1.html` |
| `slider-op2.nc` | right | 6.7 min | 30% | `slider-op2.html` |
| `slider-engrave.nc` | right, straight after `slider-op2` | 2.5 min | – | `slider-engrave.html` |
| `base-cycle.nc` | both (G54 left + G55 right) | 19.4 min (rcnc sim) | 59% | `base-cycle.html` |
| `slider-cycle.nc` | both, with the engraving | 24.1 min (rcnc sim) | 37% | `slider-cycle.html` |

Playbacks: `*-sim.html` (rCAD simulation with the machine and the vise closed on the work). Pictures:
`*-vise-iso.png`, `*-vise-front.png`.

## Blanks, shims and tools

- Saw blanks to **99.5 ± 0.5 mm**. Op 1 squares them to 98.0. Find stock accepts 98.5 to 101 (TOL 1.5).
- Aluminium: 6061 1/2 × 1 flat bar (12.7 × 25.4). Brass: C360 3/8 × 1 (9.5 × 25.4).
- **Shims for brass op 1:** two flat 1.0 mm strips about **25 × 39**, one under each slider. Put them 12 mm apart,
  centred on the bar's middle, so the two carrier bores at the middle break through between them. Both strips
  stay inside the jaws' length, so the end trims at ±49 miss them.
- Tools, with stick-outs from the ER16 nut:

  | T | Tool | Stick-out |
  |---|---|---|
  | T24 | 3 FLUTE DLC 1/4 | 24 |
  | T1 | ALUMINUM chamfer 90 | 22 |
  | T99 | YIDOFENG probe | 40 |

  The deepest T24 cut is 10.2 below the bar top (the end trims), so the nut clears by more than 13 mm.
  Flood coolant.
- Speeds:

  | Material | Cutter | rpm | Feed (mm/min) | Why |
  |---|---|---|---|---|
  | Aluminium | T24 | 22 500 | 2350 | rCAD's chatter model flagged 24k on the op 1 rough; 22.5k is its best speed. |
  | Brass | T24 | 18 000 | 1350 | Mild on purpose: the high-helix DLC cutter can pull into C360. |
  | Brass | T1 | 18 000 | 1000 | |

  In brass the adaptive links are capped at 1600 mm/min.

## Setting the vise

The centre block is the fixed jaw of both stations.

- **Left station (op 1):** the 25.4 bar is clamped between the centre block (on the part's **+X / right**) and the
  moving jaw.
  - Guidelines deviation: the guidelines put the fixed jaw on the left. Op 1 takes X from the bar's own two
    sides, so it doesn't matter which jaw is fixed. Using this station lets a batch run op 1 here and op 2 in
    the right station at the same time.
  - In the simulation the moving jaw's base is 1 hole (25 mm) further in than in the vise CAD, and the jaw
    screw closes it the rest of the way to 25.4.
- **Right station (op 2):** the fixed jaw is on the part's **−X / left**, as in the earlier jobs. The moving jaw
  closes on the finished op 1 walls: 25.0 for the base, 19.6 for the slider.
  - In the simulation the base is moved 4 holes (100 mm) in from the vise CAD's position. The CAD's 118 mm
    station isn't how your vise is set now.
  - If the right station is still set for the 39 mm test-part block, keep that hole and close the screw about
    14 mm more (base) or 20 mm more (slider).
- Clamp on the bottom 4.7 mm, as usual. Nothing stands above the jaw tops: no stops, and the shims are under
  the bar.

## Op 1 (bottom side up), left station

Lay the bar on the ledges, centred along the jaws. Both ends stick out about 2 mm past the jaws. **Brass: the two
shim strips go under it.**

1. Open `base-op1.nc` / `slider-op1.nc`, Cycle Start.
2. T99 + Find stock (TOL 1.5, DEPTH 3): jog over the middle, then Continue.
   - **Aluminium:** Z comes from the Vise reference.
   - **Brass:** Z comes from the bar's probed top, so Z0 is the bar's bottom on the shims. The guidelines allow
     a probed Z here because op 1 faces that top off.
3. T24 cuts, in this order:
   - Faces the top: 0.5 mm off the aluminium, 0.2 mm off the brass. On the brass this is the slider's sliding face,
     so the passes run along the slide (Y).
   - Roughs round both parts and the 8 mm gap between them.
   - One full-height finish pass on the walls: down to Z6.5 on the aluminium and Z4 on the brass, never lower
     than 0.3 over the jaw tops.
   - Brass only: the 6 magnet pockets.
   - **Two Ø7.5 locating bores through the carrier in the gap, at X ±5 Y0.** Op 2 locates from these, and op 2
     cuts them away.
   - Squares both ends to 98.0, full height. These are the parts' outer ends.
4. T1 cuts:
   - Both: a 0.3 edge break on the bottom outline (brass: and the pocket rims). It's kept at 0.3 because op 2
     stands the parts on the 1.9 mm ledges: that leaves **1.6 mm of land on each ledge**, and 4.4 of the
     4.7 mm grip.

## Op 2 (top side up), right station

Turn the blank over, either way (the parts sit on 1.6 mm of land on each 1.9 ledge; wipe the ledges clean first): the carrier bores are symmetric about the middle. The finished bottom goes down
on the ledges, with op 1's finished walls in the jaws. Those walls reach 5.7 mm (aluminium) / 5.3 mm (brass)
above the ledge, and the jaws grip 4.7. Push the bar against the fixed jaw and clamp. The raw band above the
walls is wider (25.4): make sure the jaws close on the finished walls, not on it.

1. Open `base-op2.nc` / `slider-op2.nc`, Cycle Start.
2. **Chip pause (BORES PAUSE):** the program stops with a message. Blow the chips out of the two carrier bores,
   check their mouths for burrs, then Cycle Start. A chip or burr in a bore can fail Find bores.
3. T99 + Find stock: a rough X/Y. Z comes from the Vise reference.
4. T99 + **Find bores**: X/Y from the two carrier bores (Ø7.5, 10 apart, TOL 0.1, turn ≤ 0.5°). This happens
   before any roughing. The carrier is located from these bores, not from a raw face.
5. Base:
   - Face to 9.
   - Rough the channel and outside to Z6 (walls leave 0.3, floor 0.2). The two bases stay joined by the gap web.
   - Finish the channel floor, with passes along the slide (Y) so the tool marks run the way the slider moves, and
     the rails.
   - The magnet pockets.
   - The rest rough below Z6. This **separates the two bases**; it's light, 0.3 deep.
   - The outline's full-height finish pass, 9 to 5.
   - T1: the 1.5 rail-top chamfers and the 0.3 edge breaks.
6. Slider:
   - Face to 8.
   - Rough the sides and the gap to Z5. The two sliders **separate during this rough**.
   - One full-height finish pass.
   - T1: the 2 mm ingot chamfer.
7. Sliders: without unclamping, open `slider-engrave.nc` and run it. It uses T1 again, the same work offset and
   no probing. To swap the pattern for a custom one or a name:
   - put a sketch on the top field (15.6 × 41 flat; keep 1 mm in from the chamfer);
   - add a `trace` op (lines) or a `vcarve` op (text) in the "Slider op2 engrave" tab of `slider-cam.json`.

**Separation risk:** after the separating cut, each part is held only by its own 45 mm of 4.7 mm grip. The cuts
after it are light: at most 0.3 mm of stock and 8% of the spindle. Both parts of a blank were finished in the
same op 1 pass, so they're the same width and the jaws hold both. Check the jaw is tight before Cycle Start.

## Production: the two-station cycle (`base-cycle.nc`, `slider-cycle.nc`)

Each cycle runs one blank's op 1 in the **left station (G54)** and the previous blank's op 2 in the **right station
(G55)** in a single program, ordered by tool, so each cutter goes in once:

| Tool | Left station (G54) | Right station (G55) |
|---|---|---|
| T99 | Find stock (raw bar) | Find stock, **BORES PAUSE** (blow out the carrier bores), Find bores |
| T24 | all of op 1 | all of op 2 |
| T1 | op 1 edge breaks | op 2 chamfers, then (sliders) the crosshatch engraving |

**The programs come straight from rCAD** (branch `multi-station`). In `slider-cam.json` the op 1 setup leads a
*job*: `with: ["Base op2 top"]` for the bases, or the op 2 and engraving setups for the sliders. Each setup keeps
its own station (`on_table`), work offset, stock and fixtures. `export_gcode` on the op 1 setup posts the whole
job, and `stations: ["Base op1 bottom"]` posts op 1 alone.

**Needs rcnc PR #14 (on `mill/ingot-slider`).** With it, each work offset keeps its own Z reference, so one setter
measurement per tool zeroes G54 and G55; the log shows `Tool touched off: ... (also G55)`. On older rcnc the right
station would cut with the previous tool's Z, so **don't run the cycle programs on it**. The single programs are
fine either way.

| | Separate programs | Cycle | Tool changes |
|---|---|---|---|
| Bases | op 1 + op 2 = 7.3 + 14.5 = 21.8 min | **19.4 min** | 6 → **3** |
| Sliders | op 1 + op 2 + engraving = 14.2 + 9.8 + about 3 = about 27 min | **24.1 min** | 7 → **3** |

The times come from the rcnc simulator, including its tool-change and probing moves. At the machine, each saved
change also saves your swap and the setter touch, about a minute each, so a cycle saves about 3–4 min plus the
wait at each change. Spindle peaks are as in the single programs (base 59%, slider 37%).

### Running batches

1. **First cycle** (right station empty): clamp a raw blank in the left station and run `base-op1.nc` /
   `slider-op1.nc`. Each is posted from the job with only op 1 and is G54 only.
2. **Every cycle after that:**
   - take the finished parts out of the right station;
   - flip the left station's blank into the right station, onto its finished bottom;
   - clamp a new raw blank in the left station (brass: on the two shim strips);
   - run `base-cycle.nc` / `slider-cycle.nc`.

   Find stock asks you to jog over each station in turn: the left first, then the right.
3. **Last cycle** (left station empty): run `base-op2.nc` / `slider-op2.nc` and then `slider-engrave.nc` (G55 only).

Within a tool, the program goes up to `G53 G0 Z0` between stations, switches the work offset, travels over at the
top and comes down over the other station.

**Checked:**
- **One rCAD simulation per job** (`base-cycle-sim.html`, `slider-cycle-sim.html`) runs both stations together on
  the machine model with both vise stations closed on their work (`DMC2-twostation-*`, from `vise_machines.py`).
  It covers the head, holder and tool against the other station's stock, part and jaws, the moves between
  stations, and travel. No problems. Pictures: `*-cycle-vise-iso.png`, `-front.png`, `-top.png`.
- **The setup sheet** for each job is `base-cycle.html` / `slider-cycle.html`.
- **rcnc simulator:** both cycle programs run to the end with all probing and three tool changes.

## Magnets

N52 (or N35/N42) Ø8 × 3, glued 0.15 below flush. For detents, every base magnet goes N up and every slider magnet
S down. Try it dry first.

## Checks

Quoted from rCAD's simulation (details in `OPS.md`). Every setup has the jaws closed on the work as machine parts
and as fixture boxes, and the toolpaths stay 0.3 over them.

| Program | Problems | Peak spindle (where) | Air | Chatter | Chips | Unfinished |
|---|---|---|---|---|---|---|
| base-op1 | none | 60%: rough at the gap, X−9.3 Y0.1, 3.25 × 2.85 | end trims 88% | ok | carrier bores: caution (0.64) | none |
| base-op2 | none | 40%: channel rough, X7.7 Y−0.3, 2.13 × 2.80 | outline finish 81% | ok | ok | none |
| slider-op1 | none | 36%: rough at the gap, X7.9 Y0.1, 2.91 × 2.95 | end trims 87% | rough: caution (0.98) | ok | none |
| slider-op2 | none | 30%: rough, X−3.4 Y0.1, 2.64 × 2.70 | outline finish 49% | ok | ok | none |
| slider-engrave | none | – | 9% | ok | ok | none |

Notes on the checks:
- **Chip caution, base op 1 carrier bores:** they are through holes in carrier that op 2 removes. Op 2's chip
  pause blows them out before Find bores, so op 1 has no pause of its own.
- **Air, end trims:** high because they trace closed loops beyond the ends to get arc-free tangential entries.
  That's about 20 s each.
- **rcnc simulator:** all four probing programs ran to the end, including Find stock, Find bores (with
  `RCNC_BORES="-5,0;5,0"`) and the tool changes:

  | Program | Simulated time |
  |---|---|
  | base-op1 | 7.3 min |
  | base-op2 | 14.9 min |
  | slider-op1 | 14.4 min |
  | slider-op2 | 9.8 min |

  `slider-engrave` needs op 2's work offset, so it wasn't run alone.
- **Simulated only.** Nothing here has been cut yet.
