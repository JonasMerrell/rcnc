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

## Op 1: the gauge's top side (`op1-top.nc`, about 26 min)

Run it straight after op 1a, **without unclamping**. It doesn't probe: G54 stays from op 1a. Its stock
model is what op 1a's passes left.

| # | Tool | Operation | Time | Spindle | Chatter | Chip |
|---|---|---|---|---|---|---|
| 1 | T24 | Face the top to Z19 (the rest of the band) | 5:31 | 20% | 0.23 | 0 |
| 2 | T24 | Slots and their end reliefs to size: traced inside the outline, 1 mm levels, **F1000** (the first lap is a full-width cut) | 0:42 | 21% | 0.75 | **0.89** |
| 3 | T24 | Rough everything above Z8: end lugs, the band beside the gauge, the dish | 12:23 | **65%** | **0.81** | 0.23 |
| 4 | T24 | Rough the walls to Z5 (extra passes ≤ 25% Ø) | 0:50 | 46% | 0.47 | 0.08 |
| 5 | T24 | **Finish** the walls and end faces Z19→Z8, one full-height pass (leads, 1.5 overlap, 0.03 last lap; seam mid −X side) | 1:12 | 23% | 0.21 | 0.06 |
| 6 | T24 | Finish the walls Z8→Z5 (lug outsides too) | 0:22 | 20% | 0.19 | 0.03 |
| 7 | T24 | Two Ø8 bores through the lugs at y ±52 (op 2 finds them) | 0:25 | 2% | | 0.60 |
| 8 | T11 | Finish the thumb dish, scallop **0.2 stepover (0.033 D)** | 3:23 | 0% | 0.04 | 0.08 |
| 9 | T1 | Engrave the top numbers 0.3 deep | 0:18 | | | |
| 10 | T1 | Chamfer 0.4 on every top edge, tip offset 1 | 0:16 | | | |

Check results:
- **rCAD simulation:** no problems. Peak spindle 65%; chatter at most 0.81, below the phone stand's 0.82.
- **rcnc simulator:** op 1a followed by op 1 runs to the end (1641 s simulated).
- **Jaws:** in the model, with 0.3 mm clearance. Nothing goes below Z5 beside the jaws.
- **Lug bores:** they break through 0.5 below Z0 at y ±52, beyond the vise body.

**Not finished, on purpose (217 mm² flagged):** the corners where each slot meets its relief keep R3.2 fillets
(an end mill's radius). Those faces don't register anything. The stop is the relief's **far wall**, which is
flat across 6.65 mm. That's wider than a tool's 6.35, so the tool's tip lands on the flat.

**Chips:** step 2 cuts 6 mm deep, 6.8 mm slots that are open at the end face, so flood should clear them. If
chips pack (the sim's 0.89 is the highest of op 1), stop after step 2 and blow them out.

**What op 1 leaves for op 2:**
- the gauge's top side finished;
- walls finished down to Z5;
- end faces finished down to Z8;
- the 3 mm carrier under the gauge;
- the end lugs (8 thick) with the two Ø8 bores.

Op 2 flips the part (top down on the ledges), grips the finished 23 mm walls, and finds the lug bores with
Find bores before any cutting. Op 2 is being planned while op 1 runs.

## Design change since the concept

An end mill can't cut a flat slot end. A Ø6.35 flat tool's tip would hit the slot's corner radii about 2 mm
before its end wall, so the gauge would read about 2 mm short.

Each slot now ends in a **13 × 7 relief**. Its far wall is cut by the tool moving sideways, so it's flat. That
needs the width, so there's **one centred slot per end per face (4 stations)**:
- top: **24** (+Y) and **21** (−Y);
- bottom: **22** (+Y) and **26** (−Y).
