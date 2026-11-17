# Stick-out gauge: how to run op 1

The stock is the 1" (25.4) 6061 bar, sawn to **120.7 mm**. It lies along Y in the **right station**:
- fixed jaw on the left (−X), jaws closed on the bar's 25.4 width;
- bottom on the vise's built-in ledges, so **Z0 = the Vise reference**;
- the jaws grip the bottom 4.7 mm;
- about 12.5 mm of bar hangs past each end of the 95.5 mm jaws, with nothing under it.

**rcnc build:** mill/stickout-gauge 85c58d3 or later. It has the listening additions, (RCNC RECORD_AUDIO),
the `.air.cutlog.json` naming, and the wait after a spindle speed change.

**Tools** (all carbide; the program headers fill rcnc's tool table):

| Tool | Stick-out |
|---|---|
| T99 probe | |
| T24 3-flute DLC 1/4 | **24** |
| T11 6 mm ball | **21** |
| T1 chamfer 90 | **22** |

At each change, use **Tap test** in the tool change window (Settings → Listening on).

## Op 1a v2: calibration passes (`op1a-calibrate-v2.nc`, about 5:30 plus tool changes) — FOR REVIEW

**STOCK 32 × 32 × 120.7** (measured). The first program, `old-do-not-use/op1a-calibrate-v1-…`, broke T24 in its
C4 (1.8 × 6 at F2400): the spindle stalled.

**Before:**
- rcnc on the **listen-guard** build. The program has `(RCNC REQUIRE_GUARD)`, so Cycle Start refuses unless
  Settings → Listening is on, the feed hold is on and the input is live. The status bar must say
  **"hold armed"**.
- **A new T24** at stick-out 24. Use Tap test at the change.
- The wall meter where you can read it.

**Run:**
1. Air cut on (+25), once: the no-load baseline. It writes `op1a-calibrate-v2.nc.air.cutlog.json`.
2. Air cut off, for real.
   - **Find stock:** jog the probe over the **unstepped** part of the top (x < 10). The +X side has the first
     run's 4.75 mm step at Z26.
   - Find stock sets X/Y; Z stays from the Vise reference.
3. Each pass shows **"Pass N: …"** in the message line, then the spindle runs 3 s in air, then the cut.
   - Read the meter during the 3 s of air and during the cut, and call out "pass N = W".
   - Afterwards put them in `wall_watts.json`: `{"air": {"24000": W, "21000": W, …}, "C9": W, …}`.
4. If the bog guard holds ("FEED HELD: spindle bogging"):
   - **stop the program** (don't Cycle Start through it);
   - note the pass. That's the measured limit.

**Passes:** the table is `calibration.md`, with columns for step over, depth, feed, rpm, MRR, model W, proven
and wall W.
- Chips stay at the proven 0.033 mm/tooth. Each cut runs twice back to back in the same direction, about 6 s
  of steady cutting for the meter.
- Max 472 W by the pull-out model; nothing over 500 W.
- 1–8: clean the first run's torn C4 end (air except there).
- 9–10: baseline 0.6 × 6 F2400 (proven, new tool).
- 11–14: 0.2 and 0.3 × 12.
- 15–22: rpm sweep, 0.3 × 12 at 21 / 18 / 15 / 12k at the same chip.
- 23–26: 0.8 × 6 and 0.4 × 12.
- 27–28: the labelled thin-chip comparison, 0.6 × 12 F1200.
- 29–32: 1.0 × 6 and 0.45 × 12, guard armed.

**After:** copy the `.cutlog.json` files (and `.wav`) here and run:

    python3 analyse.py op1a-calibrate-v2.nc.cutlog.json op1a-calibrate-v2.nc.air.cutlog.json predicted.json wall_watts.json

## Op 1: the gauge's top side (NOT YET RE-PLANNED for the 32 mm bar and the pull-out model: do not run) (`op1-top.nc`, about 24 min plus one chip pause)

Run it straight after op 1a, **without unclamping**. It doesn't probe: G54 stays from op 1a. Its stock model
is what op 1a's passes left.

| # | Tool | Operation | Time | Spindle | Chatter | Chip |
|---|---|---|---|---|---|---|
| 1 | T24 | Face the top to Z19 (the rest of the band) | 5:31 | 20% | 0.23 | 0 |
| 2 | T24 | Rough the slots and reliefs: traced inside, 1 mm levels, **F1000**, 0.2 left on the walls, floors to depth | 0:43 | 21% | 0.73 | 0.91 |
| — | | **CHIP PAUSE**: spindle and coolant stop, rcnc says "Blow the chips out of the two slots and their end reliefs". Blow them out, then Cycle Start. | | | | |
| 3 | T24 | Rough everything above Z8: end lugs, the band beside the gauge, the dish | 12:25 | **65%** | **0.81** | 0.22 |
| 4 | T24 | Rough the walls to Z5 (extra passes ≤ 25% Ø) | 0:53 | 46% | 0.47 | 0.08 |
| 5 | T24 | **Finish the slot walls and the relief stop walls**: one full-depth 6 mm pass round their outline | 0:09 | 45% | 0.38 | 0.16 |
| 6 | T24 | **Finish** the walls and end faces Z19→Z8, one full-height pass (leads, 1.5 overlap, 0.03 last lap; seam mid −X side) | 1:12 | 23% | 0.21 | 0.05 |
| 7 | T24 | Finish the walls Z8→Z5 (lug outsides too) | 0:22 | 20% | 0.19 | 0.03 |
| 8 | T24 | Two Ø8 bores through the lugs at y ±52 (op 2 finds them) | 0:25 | 2% | | 0.60 |
| 9 | T11 | Finish the thumb dish, scallop **0.2 stepover (0.033 D)** | 0:50 | 0% | 0.04 | 0.17 |
| 10 | T1 | Engrave the numbers 0.3 deep | 0:18 | | | |
| 11 | T1 | Edge break 0.3 on the two bore mouths (top side) | 0:08 | | | |
| 12 | T1 | Edge break 0.3 on every edge of the gauge's top: outline, slots, reliefs, dish rim | 0:16 | | | |

**The slots:**
- Slots are now **7.2 wide** (the tool cradles in them; 0.2 rough allowance each side).
- Each relief is **14 × 8** with R3.6 inside corners. That's bigger than the tool's 3.175 plus the 0.2
  allowance, so the corners finish too.
- The far wall (the stop) is flat across **6.8 mm**, wider than a tool's 6.35.

**Edges:**
- Every edge op 1 makes on the gauge gets a 0.3 edge break, not 0.4. These top edges become the bottom in
  op 2, standing on the 1.9 mm vise ledges, so 1.6 mm of land is left on each ledge.
- The engraving needs none.

**Chips:** step 2 has the highest chip index (0.91). It's a full-width cut 6 deep, so the pause after it stays.
The slots are open at the end faces, so flood should clear most of it.

**Bores:** chip index 0.60. Before op 2:
- blow out both lug bores;
- deburr their **bottom ends** by hand (a deburring tool or countersink, lightly). That's where the bores
  broke through in op 1, and it's the side op 2 probes from.

Op 2's program also says so before Find bores.

Check results:
- **rCAD simulation:** no problems. Peak spindle 65%; chatter at most 0.81.
- **rcnc simulator:** op 1a followed by op 1 runs to the end (1552 s), including the chip pause.

**Unfinished flag (200 mm² at one convex corner):** the sim flags the corner where a slot wall meets its
relief's near wall (x −3.6, y −31). The finishing pass in step 5 wraps that corner with an R3.175 arc (the
`G3 X-3.089 Y-27.84 … X-0.425 Y-32.1` moves). The flag reads the wall's full height (5.8 mm), so it's the
sim's grid at a sharp corner, not stock left.

**What op 1 leaves for op 2:**
- the gauge's top side finished;
- walls finished down to Z5;
- end faces finished down to Z8;
- the 3 mm carrier under the gauge;
- the end lugs (8 thick) with the two Ø8 bores.

## Op 2: the gauge's bottom side (`op2-bottom.nc`, about 6:30 plus two stops)

1. **Unclamp and flip** the part: op 1's finished top goes down on the ledges.
   - Turn it over either way (about X or about Y). The bores and the part are symmetric about the middle, so
     both land the same.
   - Close the jaws on the **23 mm finished walls**, gripping 4.7 mm.
   - The lugs stand up past the jaw ends.
2. Cycle Start. **The program stops first**: blow the chips out of both lug bores and deburr their upper ends
   (where they broke through in op 1).
3. `(RCNC Z_REFERENCE Vise)`, then T99 **Find bores**: `A=0,-52 B=0,52 D=8 TOP=19 DEPTH=4 TOL=0.1 ROT=0.2`.
   This sets X/Y from the bores **before any cutting**.
   - `TOP=19` is the lugs' top here (the bar's bottom): the ball touches 5 mm down, in the middle of the 8 mm
     lug.
   - If a check fails, rcnc holds the program with Probe again.
4. T24, with a Tap test, then T1:

| # | Tool | Operation | Time | Spindle | Chatter | Chip |
|---|---|---|---|---|---|---|
| 1 | T24 | Face the 3 mm carrier off (Z19 → Z16): **finishes the gauge's bottom** | 2:20 | 20% | 0.23 | 0 |
| 2 | T24 | Rough the bottom slots and reliefs (1 mm levels, F1000, 0.2 left) | 0:43 | 21% | 0.75 | 0.91 |
| — | | **CHIP PAUSE**: blow the chips out of the slots and reliefs, then Cycle Start | | | | |
| 3 | T24 | **Lugs off: 22 straight passes across X**, 1.5 mm strips, 5.5 deep, from each lug's free end in toward the gauge (11 per lug, each its own operation so the order is fixed; each climbs). The last pass stops 0.3 off the end face. | about 1:10 | 50–54% | 0.55 | |
| 4 | T24 | Rough what's left down to Z5: the thin side strips on the walls | 0:24 | 7% | 0.16 | 0.06 |
| 5 | T24 | Finish the slot walls and the relief stop walls, one full-depth pass | 0:08 | 45% | 0.38 | 0.16 |
| 6 | T24 | Finish the walls and end faces Z16 → Z10.5, one full-height pass (meets op 1's finish; seam mid −X) | 0:25 | 16% | 0.18 | 0.04 |
| 7 | T1 | Engrave the bottom numbers (22, 26) 0.3 deep | 0:13 | | | |
| 8 | T1 | Edge break 0.3 on every edge of the bottom face: outline (includes the end faces' bottom edges), slots, reliefs | 0:12 | | | |

**Nothing comes loose:**
- The lugs sit on air: op 1 left them 8 thick, and below them is open.
- Step 3 cuts them away as chips, strip by strip, from the free end in. What's left of a lug is always the
  part still joined to the gauge.
- The bores only split a strip into two pieces that both stay joined inward.
- The side strips are 1.2 × 2 mm slivers along the walls, still joined along their whole length when step 4
  and the wall finish take them.

Check results:
- **rCAD simulation:** op 2 starts from what op 1 really leaves (lugs and carrier included). No problems;
  unfinished 1 mm² (the slot-relief corner flag, as in op 1).
- **rcnc simulator:** runs to the end with the lug bores in the bar (583 s). Find bores, started 0.4 / 0.3 mm
  off on purpose, reads both bores 104.000 apart and moves G54 onto them.

**Sharp edges left for a file (vertical edges: a chamfer mill can't reach them):**
- the gauge's **4 outside vertical corners**;
- the **8 vertical edges at the slot mouths**, where each slot's side walls meet the end faces (2 per slot,
  4 slots).

Every horizontal edge is broken by op 1 or op 2.

## Design change since the concept

An end mill can't cut a flat slot end. A Ø6.35 flat tool's tip would hit the slot's corner radii about 2 mm
before its end wall, so the gauge would read about 2 mm short.

Each slot now ends in a **14 × 8 relief**. Its far wall is cut by the tool moving sideways, so it's flat. That
needs the width, so there's **one centred slot per end per face (4 stations)**:
- top: **24** (+Y) and **21** (−Y);
- bottom: **22** (+Y) and **26** (−Y).

## After op 1a: what the transducer heard

Copy `op1a-calibrate.nc.cutlog.json` and `op1a-calibrate.nc.air.cutlog.json` from beside the program on the
mill PC (plus the `.wav` files if you want them kept), then:

    python3 analyse.py op1a-calibrate.nc.cutlog.json op1a-calibrate.nc.air.cutlog.json predicted.json

Per pass C1–C16 it lists:
- rCAD's predicted spindle % and watts;
- the speed droop heard (with the air run's own droop taken off);
- the tooth-pass and overall vibration level over the air run;
- how often chatter was heard and at what frequency;
- rCAD's chatter index.

Then it fits:
- spindle efficiency, from droop (rCAD now uses 0.78);
- how straight droop follows predicted power;
- the chatter scale (rCAD now 0.078), with a warning if the index can't separate clean passes from
  chattering ones.

`predicted.json` comes from rCAD's simulation of op 1a (`predicted.py`). rCAD's own `import_cutlog` can read the
same log too.
