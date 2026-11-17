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

## Op 1a: calibration passes (`op1a-calibrate.nc`, about 3:00 plus tool changes)

1. Settings → Listening: input on. Check the level during the first air pass: it must not clip.
2. **Run it once with Air cut on (+25 mm).** This is the no-load baseline of every pass at its own speed
   and feed. rcnc writes `op1a-calibrate.nc.air.cutlog.json` and `.air.wav`.
   - Find stock and the tool changes happen for real.
3. **Air cut off, run it for real.** Writes `op1a-calibrate.nc.cutlog.json` and `.wav`.

What the program does:
- `(RCNC Z_REFERENCE Vise)`, then T99 **Find stock**: `SIZE=25.4,120.7,25.4 DEPTH=8 KEEPZ=1`. It sets G54
  X/Y; Z stays from the Vise reference (never from the raw top).
- T24: **16 straight climb passes along Y**, each from y −67 to +67. That's the tool radius plus 3.5 mm past
  each sawn end, so the plunge and the exit are in air.
- Passes are named C1–C16 in the program and the cut log. The table is `calibration.md`.

The rCAD stock simulation of these passes, with rCAD's cutting model:

| Passes | Cut | Spindle | Chatter index |
|---|---|---|---|
| C1–C4 | width ladder | 35 / 40 / 55 / **75**% | 0.38 / 0.38 / 0.53 / 0.80 |
| C5, C6 | feed ladder 1800 / 3000 | 45 / 71% | |
| C7–C11 | rpm sweep 18–24k | 40–45% | 0.34–0.49 |
| C12 | repeat of C3 | 55% | |
| C13, C14 | depth 4.5 / 3 | 45 / 28% | |
| C15, C16 | 12 deep: 0.3, and your 0.6 at 2500 | 38 / 62% | |

- No problems in the sim; chip fill ≤ 0.25.
- The rcnc simulator runs it to the end: Find stock read the top at Z25.400.
- The passes only take the top 6 mm band and the 1.2 mm strip outside the +X wall.

## Op 1: the gauge's top side (`op1-top.nc`, about 24 min plus one chip pause)

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

## Design change since the concept

An end mill can't cut a flat slot end. A Ø6.35 flat tool's tip would hit the slot's corner radii about 2 mm
before its end wall, so the gauge would read about 2 mm short.

Each slot now ends in a **14 × 8 relief**. Its far wall is cut by the tool moving sideways, so it's flat. That
needs the width, so there's **one centred slot per end per face (4 stations)**:
- top: **24** (+Y) and **21** (−Y);
- bottom: **22** (+Y) and **26** (−Y).
