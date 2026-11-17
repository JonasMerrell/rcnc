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
1. **Pick the carbide out of the torn end.** The broken T24's tip may still be stuck in the first run's C4 cut
   at the +X end. Look in the cut and its end with a light, and pick out any fragment (a magnet won't
   reliably hold carbide; use a scriber or pick). A new tool hitting a carbide chip breaks too.
2. rcnc on the **listen-guard** build (mill/stickout-gauge 02c350c or later).
3. **Home** the machine first.
4. **A new T24 at stick-out 24.** Use Tap test at the change.
5. **Settings → Listening:** input on, **feed hold on**. The status bar must say **"hold armed"** (green)
   before Cycle Start. The program has `(RCNC REQUIRE_GUARD)`, so Cycle Start refuses without it.
6. The wall meter where you can read it.

**Run:**
1. Air cut on (+25), once: the no-load baseline. It writes `op1a-calibrate-v2.nc.air.cutlog.json`.
2. Air cut off, for real.
   - **Find stock:** the bar is stepped. Only **X −16 to +11.2** is still full height (Z32); past x +11.2
     the first run cut it down to Z26. Jog the probe over the **middle of the full-height part, about
     X −2** (a few mm left of the bar's middle), a few mm above it, and press Continue.
   - The side touches are at Z24, below the step's floor, so they still find the full 32 width.
   - Find stock sets X/Y; Z stays from the Vise reference.
3. **Wall watts.** Write them down as you go:
   - **air first:** each pass shows **"Pass N: …"**, then the spindle runs 3 s in air. Read the meter in the
     3 s of air at each new rpm (24k, 21k, 18k, 15k, 12k);
   - **then each pass:** read it during the cut and call out "pass N = W".
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

## Op 1: the gauge's top side (`op1-top.nc`, about 52 min plus one chip pause)

Re-planned for the **32 × 32 bar** and the spindle pull-out model (645 W stall at 24k):
- every bulk cut is a **straight pass at ≤ 1 mm stepover**;
- **peak 432 W** by rCAD's sim; nothing over 450.

Run it straight after op1a v2, **without unclamping**:
- It doesn't probe: G54 stays from op 1a.
- Its stock model is what op1a v2 leaves: z 32 in the middle, the +X side at z 26 in to x +6.4, the −X side at
  z 20 in to x −9.7.
- It has `(RCNC REQUIRE_GUARD)`: keep Listening on with the feed hold armed.

| # | Tool | Operation | Passes | Time | Peak W (spindle %) |
|---|---|---|---|---|---|
| 1 | T24 | **Band z32 → 26** (the middle, x +6.4 → −9.7): 0.85 × 6 strips, **F2100**, climb along +Y | 19 | 1:54 | 427 (66%) |
| 2 | T24 | **Band z26 → 19.2** (x +16 → −9.7): 0.73 × 6.8, **F2100** | 35 | 4:05 | 423 (66%) |
| 3 | T24 | Band −X side z20 → 19.2: 0.9 × 0.8 | 7 | 0:42 | 63 |
| 4 | T24 | **Finish the gauge's top**: face z19.2 → 19, one 0.2 level | 1 | 0:54 | 47 |
| 5 | T24 | **Lugs** z19 → 8: lines across X, 0.89 strips from each free end in, two 5.5 levels each; last line 0.3 off the end face | 72 | 7:12 | 432 (67%) |
| 6 | T24 | **Side strips** beside the gauge z19 → 8.5: five 0.84 strips a side from the bar's edge in, two 5.25 levels | 20 | 2:50 | 413 (64%) |
| 7 | T24 | Rough the thumb dish (adaptive, 0.64 stepover, 1 mm levels) | 1 | 7:35 | 299 |
| 8 | T24 | Rough the slots and reliefs: traced inside, 1 mm levels, F1000, 0.2 left on the walls | 1 | 0:46 | 242 |
| — | | **CHIP PAUSE**: spindle and coolant stop. Blow the chips out of the two slots and their reliefs, then Cycle Start. | | | |
| 9 | T24 | Rough the walls z8.5 → 5 beside the jaws (adaptive, 0.64 stepover, 1.2 levels, 0.3 left) | 1 | 10:17 | 205 |
| 10 | T24 | **Finish the slot walls and relief stop walls**: one full-depth 6 mm pass at **F1200** (the relief corners carry more) | 1 | 0:15 | 321 |
| 11 | T24 | **Finish** the walls and end faces z19 → 8, one full-height pass (seam mid −X side) | 1 | 0:47 | 254 |
| 12 | T24 | Finish the walls z8 → 5 (lugs' outsides too), F1200 | 1 | 0:44 | 270 |
| 13 | T24 | Two Ø8 bores through the lugs at y ±52 (op 2 finds them) | 1 | 0:28 | 222 |
| 14 | T11 | Finish the thumb dish: scallop, **0.2 stepover (0.033 D)** | 1 | 8:01 | 4 |
| 15 | T1 | Engrave the numbers 0.3 deep | 1 | 0:28 | |
| 16 | T1 | Edge break 0.3 on the two bore mouths | 1 | 0:08 | |
| 17 | T1 | Edge break 0.3 on every edge of the gauge's top: outline, slots, reliefs, dish rim | 1 | 0:16 | |

**Loads:**
- The band passes are F2100, not F2400. rCAD's sim grid (0.134 cells) reads the 0.85 / 0.73 strips about 0.1
  wider; at F2400 the sim read 472 W. The plan model (true width) gives 431 W at F2400 and about 400 at F2100.
- Each pass runs y −67 → +67: plunge and exit are in air past the sawn ends.
- Every straight pass climbs, with the material on the tool's left.
- The bog guard is the backstop on all of it.

**Chips:** chip index at most 0.21 on the straight passes. The slot rough is 0.71, so the pause stays after it.
Bores 0.50.

Check results:
- **rCAD simulation:** ok, no problems, peak 432 W (67%).
  - It flags 792 mm² "unfinished" at the slot/relief convex corner (x +3.6, y −31). The slot finish wraps that
    corner with an R3.17 arc (`G3 X0.425 Y-32.098 …`), so it's the sim's grid at a sharp corner, as before.
- **rcnc simulator:** op1a v2 then op 1, on the stepped bar (Find stock with the probe at x −2.4), runs to the
  end in 3108 s. Find stock read 32.000 × 120.700, top Z32. Chip pause and tool changes T99 → T24 → T11 → T1 as
  planned. The lowest the tool goes is Z0.5 (the bores).

**The slots:**
- Slots are now **7.2 wide** (the tool cradles in them; 0.2 rough allowance each side).
- Each relief is **14 × 8** with R3.6 inside corners. That's bigger than the tool's 3.175 plus the 0.2
  allowance, so the corners finish too.
- The far wall (the stop) is flat across **6.8 mm**, wider than a tool's 6.35.

**Edges:**
- Every edge op 1 makes on the gauge gets a 0.3 edge break, not 0.4. These top edges become the bottom in
  op 2, standing on the 1.9 mm vise ledges, so 1.6 mm of land is left on each ledge.
- The engraving needs none.

**Bores:** before op 2:
- blow out both lug bores;
- deburr their **bottom ends** by hand (a deburring tool or countersink, lightly). That's where the bores
  broke through in op 1, and it's the side op 2 probes from.

Op 2's program also says so before Find bores.

**What op 1 leaves for op 2:**
- the gauge's top side finished;
- walls finished down to Z5;
- end faces finished down to Z8;
- the 3 mm carrier under the gauge;
- the end lugs (8 thick) with the two Ø8 bores.

## Op 2: the gauge's bottom side (BEING RE-PLANNED for the 32 mm bar: not posted yet)

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
