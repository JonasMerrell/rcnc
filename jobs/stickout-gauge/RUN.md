# Stick-out gauge: how to run op 1

The stock is a **32 × 32** 6061 bar (measured; Find stock read 31.90 × 120.45), sawn to **120.7 mm**. It lies
along Y in the **right station**:
- fixed jaw on the left (−X), jaws closed on the bar's 32 width;
- bottom on the vise's built-in ledges, so **Z0 = the Vise reference**;
- the jaws grip the bottom 4.7 mm;
- about 12.5 mm of bar hangs past each end of the 95.5 mm jaws, with nothing under it.

**rcnc build:** fix/guard-arm **af90c74** or later: the bog guard that hears the spindle at every speed, no level
check at Cycle Start (status "hold ready", then "hold armed" once the spindle has run in air), (RCNC
REQUIRE_GUARD) and (RCNC RECORD_AUDIO).

**Tools** (all carbide; the program headers fill rcnc's tool table):

| Tool | Stick-out |
|---|---|
| T99 probe | |
| T24 3-flute DLC 1/4 | **24** |
| T11 6 mm ball | **21** |
| T1 chamfer 90 | **22** |

At each change, use **Tap test** in the tool change window (Settings → Listening on).

## Op 1a v6: a 9k mini-ladder with F004 = 150 (`op1a-calibrate-v6.nc`, about 2:00 plus a tool change)

v5 (2026-11-18, F004 = 200) ran passes 1-7 at 12k: 484 W 3.8% sag, 590 W 5.2%, 676 W 5.7%, 759 W about 6.9% (held at
6%), 839 W about 9% (held), no runaway. The VFD is now at **F004 = 150** (no-load at 9k: 108 V, 2.7 A). v6 cuts the
same model watts as v5's passes 3-6, at **S9000**, to compare: four single passes 12 deep (Z32 -> Z20) from the -X
wall (x -3.75), 1.0 mm each; the wall ends at x +0.25, a 6.15 mm core left. All waste above the gauge.

| Pass | Step over | Feed | Model W | N·m | MRR cm³/min | Chip (max) mm | Tool stress MPa | Strip x |
|---|---|---|---|---|---|---|---|---|
| 1 | 1.0 | F1270 | 484 | 0.51 | 15.2 | 0.034 | 226 | -3.75 .. -2.75 |
| 2 | 1.0 | F1650 | 589 | 0.63 | 19.8 | 0.045 | 276 | -2.75 .. -1.75 |
| 3 | 1.0 | F1980 | 676 | 0.72 | 23.8 | 0.053 | 316 | -1.75 .. -0.75 |
| 4 | 1.0 | F2310 | 759 | 0.80 | 27.7 | 0.062 | 355 | -0.75 .. +0.25 |

- At 9k the same watts are 1.33 x the torque of 12k, so the force and the chip are bigger: chips 0.034-0.062 mm
  (slightly over 0.05 in the last two: fine for carbide in 6061), the tool at most 355 of 800 MPa.
- **If F004 = 150 helps:** fewer rpm of slip per watt than v5 (v5 at 12k: 484 W 443 rpm, 590 W 595, 676 W 680).
- **The hold at 9k:** 6% is only about 520 rpm of slip, so it will stop early. Same rule as v5: a hold that says
  "over 6.0%" = stop the program; "still slowing down" in a pass's first second = one Cycle Start allowed, a second
  hold on that pass = stop.

**Before:** **F004 = 150** on the VFD. Don't unclamp. Home, check **G54 X/Y = machine X120.472 Y387.240**.
Listening on, hold on.

**After:** copy `op1a-calibrate-v6.nc.cutlog.json`, `.wav` and rcnc's `log.txt`.

## Op 1a v5 (RAN 2026-11-18, passes 1-7; in old-do-not-use/): the 12k ladder

v4 (2026-11-18) repeated run 1's 12k cut with the VFD's **F004 = 200**: it didn't bog at all. v5 finds how much the
spindle carries at 12000 rpm now: ten single passes, all at **S12000**, 12 deep (Z32 -> Z20) from the bar's -X wall
(x -9.7) in, each about 80-120 W (rCAD's model) heavier than the one before, from 247 W to 1130 W. Everything cut is
waste above the gauge (Z3-Z19): op 1 later meets only less material. A 6.3 mm wide core is left (x +0.1 to +6.4).

| Pass | Step over | Depth | Feed | Model W | N·m | MRR cm³/min | Chip (max) mm | Force N | Tool bend mm | Tool stress MPa | Strip x |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.45 | 12 | F1200 | 247 | 0.20 | 6.5 | 0.017 | 62 | 0.006 | 87 | -9.70 .. -9.25 |
| 2 | 0.7 | 12 | F1200 | 364 | 0.29 | 10.1 | 0.021 | 91 | 0.009 | 128 | -9.25 .. -8.55 |
| 3 | 0.8 | 12 | F1500 | 484 | 0.39 | 14.4 | 0.028 | 121 | 0.012 | 170 | -8.55 .. -7.75 |
| 4 | 1 | 12 | F1500 | 590 | 0.47 | 18.0 | 0.03 | 148 | 0.015 | 207 | -7.75 .. -6.75 |
| 5 | 1 | 12 | F1800 | 676 | 0.54 | 21.6 | 0.036 | 169 | 0.017 | 237 | -6.75 .. -5.75 |
| 6 | 1 | 12 | F2100 | 759 | 0.60 | 25.2 | 0.042 | 190 | 0.019 | 266 | -5.75 .. -4.75 |
| 7 | 1 | 12 | F2400 | 839 | 0.67 | 28.8 | 0.049 | 210 | 0.021 | 294 | -4.75 .. -3.75 |
| 8 | 1.15 | 12 | F2400 | 949 | 0.76 | 33.1 | 0.051 | 238 | 0.024 | 333 | -3.75 .. -2.60 |
| 9 | 1.3 | 12 | F2400 | 1058 | 0.84 | 37.4 | 0.054 | 265 | 0.026 | 371 | -2.60 .. -1.30 |
| 10 | 1.4 | 12 | F2400 | 1130 | 0.90 | 40.3 | 0.055 | 283 | 0.028 | 396 | -1.30 .. +0.10 |

- Chips stay 0.017-0.055 mm (3 flutes, F = 1200-2400).
- The tool is not the limit: at the top step about 400 MPa of its 800, 0.03 mm of bend (rCAD's model, 24 out,
  the force at mid-depth). The ladder is capped by the bar's width (the 6 mm core), at 1130 W.
- Expected: unknown. The estimate for the pull-out at 12k now is between 650 and 1300 W by the model; at 24k it
  stays at about 645 W (the drive can't give more than 110 V there).

**Before:**
- **The VFD must have F004 = 200.**
- **Don't unclamp the bar**: v5 uses v2's G54. Home, check **G54 X/Y = machine X120.472 Y387.240**.
- rcnc on fix/guard-arm (this build); Settings -> Listening on, hold on: "hold ready", then "hold armed".

**Run:** each pass shows "Pass N: ... [W by the model]", 3 s in air, then the cut.
**If the hold trips: do NOT Cycle Start again.** Stop the program and note the pass: that's the limit.

**After:** copy `op1a-calibrate-v5.nc.cutlog.json`, `.wav` and rcnc's `log.txt`.

## Op 1a v4 (RAN 2026-11-18; in old-do-not-use/): the VFD before/after test

v2 run 1 (2026-11-18) ran passes 1-20; the bog guard held in pass 21, 0.3 x 12 at F1200 **S12000**, at 8% sag
(about 700 rpm of slip on an air speed of about 11 640). The VFD (H100) was then found to give the spindle half
its flux: **F004 (reference frequency) 400 -> 200** since then (in air: 12k now 200 Hz 108 V 1.6 A, was 55 V
0.63 A; 24k unchanged, the drive can't give more than 110 V). v4 repeats the cut that bogged, unchanged, then
finishes the bar at 24k:
- **passes 21/22 exactly as in v2**: 0.3 x 12 F1200 at S12000. Pass 21 runs in air down to Y-8.1, where run 1
  stopped, then cuts;
- **passes 23-32 as planned, at 24k** (0.8 x 6, 0.4 x 12, the thin-chip 0.6 x 12 at F1200, 1.0 x 6, 0.45 x 12;
  peak 499 W by the model, rCAD's sim 472 W): the bar ends as op 1 (`op1-top.nc`) expects, and they show whether
  24k really is unchanged (set 29/30 and 31/32 against run 1's 13/14: 0.3 x 12 sagged 2.7% at 24k).
- Every speed change is followed by 3 s in air (each pass's "Pass N" message and G4 P3): the bog guard learns each
  speed's no-load speed there.

**What to expect in 21/22:**
- **if the fix works:** the motor at full flux slips about a quarter as much for the same torque: about 175 rpm,
  **about 1.5% sag**, steady. No hold.
- **if it doesn't:** about 8% and still falling, as in run 1: the hold trips (6% for 0.3 s, or over 4% and still
  falling for 0.5 s). Then stop the program and send the logs; don't Cycle Start through it.

**Before:**
- **The VFD must have F004 = 200** for this run (check on the drive before starting; it was 400 in run 1).
- **Don't unclamp the bar.** v4 has no Find stock (the bar's -X side is cut down to Z20, so it can't measure the
  32 width any more): it uses v2's G54. rcnc keeps work offsets in its data folder (state.json) across restarts
  and rebuilds; they are machine coordinates, good again once the machine is homed.
- Home the machine, then check **G54 X/Y = machine X120.472 Y387.240** (what v2 run 1's Find stock set,
  2026-11-18 01:06; Z from the Vise reference). If it isn't, or the bar has been unclamped, stop and ask for a
  re-plan.
- rcnc on fix/guard-arm (this build): it hears the spindle at every speed; status "hold ready", then "hold armed"
  once the spindle has run in air.
- T24 stays the same tool (re-measured at the tool change).
- If you can: reset the wall meter's max just before pass 29 (the 1.0 x 6), and note it at the end.

**Run:** each pass shows "Pass N: ...", 3 s in air, then the cut. If the hold trips, stop and note the pass.

**After:** copy `op1a-calibrate-v4.nc.cutlog.json` and `.wav` from beside the program.

## Op 1a v3 (superseded by v4, not run; in old-do-not-use/)

## Op 1a v2 (RAN 2026-11-18, passes 1-20; now in old-do-not-use/): calibration passes

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

## Op 1: the gauge's top side (`op1-top.nc`, about 50 min plus one chip pause)

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
| 9 | T24 | Rough the walls z8.5 → **5.5** beside the jaws (adaptive, 0.64 stepover, 1.5 levels, 0.3 left) | 1 | 7:14 | 258 |
| 10 | T24 | **Finish the slot walls and relief stop walls**: one full-depth 6 mm pass at **F1200** (the relief corners carry more) | 1 | 0:15 | 321 |
| 11 | T24 | **Finish** the walls and end faces z19 → 8, one full-height pass (seam mid −X side) | 1 | 0:47 | 254 |
| 12 | T24 | Finish the walls z8 → **5.5** (lugs' outsides too), F1200 | 1 | 0:43 | 225 |
| 13 | T24 | Two Ø8 bores through the lugs at **y ±53.5** (op 2 finds them), to Z−0.5 | 1 | 0:28 | 190 |
| 14 | T11 | Finish the thumb dish: scallop, **0.2 stepover (0.033 D)** | 1 | 8:55 | 5 |
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

**Jaw clearance:**
- The jaws grip the bar's bottom 4.7 (4.13 in the CAD), so nothing in op 1 goes below **Z5.5** within
  y ±47.75 (the jaws' length) with the tool past x ±12.8. That's checked on the posted G-code.
- The walls are roughed and finished down to Z5.5 only. Op 2 takes what's left beside them, 1 mm past it.
- rCAD's fixture clearance is 0.8, so its sim would flag any tool within 0.8 of the jaw tops.
- **The bores are the only cuts below Z5.5** (to Z−0.5, through the lugs).
  - They moved to **y ±53.5**: the tool's edge stays at |y| ≥ 49.5, 1.75 past the jaw ends (47.75). At ±52
    it was 0.25.
  - Under the lugs there is nothing: they overhang the jaw ends.

Check results:
- **rCAD simulation:** ok, no problems, peak 432 W (67%), 49:52.
- **rcnc simulator:** op1a v2 then op 1, on the stepped bar (Find stock with the probe at x −2.4), runs to the
  end in 3009 s. Find stock read 32.000 × 120.700, top Z32. Chip pause and tool changes T99 → T24 → T11 → T1 as
  planned.

**"Unfinished" 792 mm²: what it is.** I dumped the cells rCAD counts (part surface with more than 0.1 of stock
left, seen from above):

| Area | Where | Stock left | What it is |
|---|---|---|---|
| 786 mm² | x ±11.5..16, y −44..+44, on part Z3 | 2.5 | The **carrier's top beside the gauge**: the bar from Z3 to Z5.5 stays there on purpose. It's the jaws' grip and not a gauge surface. Op 2's carrier passes and side strips remove it (to op 1's Z6.5). |
| 5 mm² | the four corners where the lugs meet the side zone, y ±44 | up to 5 | The R3.175 fillet a flat tool leaves in a concave corner on the lug stub (waste). Op 2's lug passes cut it away. |
| 0.6 mm² | x +3.6, y −31, Z13 | 5.7 | The slot/relief convex corner: grid only. The slot finish wraps it with an R3.17 arc (`G3 X0.425 Y-32.098 …`). |

Every gauge surface op 1 machines is finished by a named step: top (4), slot walls and stops (10), walls and
end faces Z19 → 8 (11) and Z8 → 5.5 (12), dish (14).

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
- walls finished down to Z5.5 (op 2 finishes the rest);
- end faces finished down to Z8;
- the 3 mm carrier under the gauge;
- the end lugs (8 thick) with the two Ø8 bores at y ±53.5;
- beside the walls, the bar from Z0 to Z5.5 (the jaws' grip).

## Op 2: the gauge's bottom side (`op2-bottom.nc`, about 11:40 plus two stops)

Re-planned for the 32 mm bar on the pull-out model, like op 1: straight passes at ≤ 1 mm stepover, **peak
393 W** by rCAD's sim. It has `(RCNC REQUIRE_GUARD)`.

1. **Unclamp and flip** the part: op 1's finished top goes down on the ledges.
   - Turn it over about X (end for end along Y stays). The bores and the part are symmetric, so either way
     lands the same.
   - Close the jaws on the **23 mm finished walls**, gripping 4.7.
   - The lugs stand up past the jaw ends.
2. Cycle Start. **The program stops first**: blow the chips out of both lug bores and deburr their upper ends
   (where they broke through in op 1).
3. `(RCNC Z_REFERENCE Vise)`, then T99 **Find bores**: `A=0,-53.5 B=0,53.5 D=8 TOP=19 DEPTH=4 TOL=0.1 ROT=0.2`.
   This sets X/Y from the bores **before any cutting**. If a check fails, rcnc holds with Probe again.
4. T24 (Tap test), then T1:

| # | Tool | Operation | Passes | Time | Peak W (spindle %) |
|---|---|---|---|---|---|
| 1 | T24 | **Carrier off** z19 → 16.2: 1.0 × 2.8 strips across the whole bar, climb | 32 | 2:08 | 248 (38%) |
| 2 | T24 | **Finish the gauge's bottom**: face z16.2 → 16, one 0.2 level | 1 | 0:53 | 47 |
| 3 | T24 | Rough the bottom slots and reliefs (1 mm levels, F1000, 0.2 left) | 1 | 0:43 | 242 |
| — | | **CHIP PAUSE**: blow the chips out of the slots and reliefs (chip index 0.92), then Cycle Start | | | |
| 4 | T24 | **Lugs off**: lines across X, 0.89 strips 5.5 deep from each lug's free end in, each climbing; the last stops 0.3 off the end face | 36 | 1:48 | 393 (61%) |
| 5 | T24 | **Side strips** beside the walls z16 → 12.5: what op 1 left over the carrier (it stopped at op 1's Z5.5 = here Z13.5), 1 mm past it, five 0.84 strips a side | 10 | 0:50 | 232 (36%) |
| 6 | T24 | Rough whatever is left down to Z5 (adaptive, 0.64 stepover, rest) | 1 | 0:55 | 113 |
| 7 | T24 | Finish the slot walls and relief stop walls, one full-depth pass at **F1200** | 1 | 0:11 | 321 |
| 8 | T24 | **Finish** the walls and end faces z16 → 10.5, one full-height pass (meets op 1's finish, 3 mm overlap; seam mid −X) | 1 | 0:25 | 169 |
| 9 | T1 | Engrave the bottom numbers (22, 26) 0.3 deep | 1 | 0:13 | |
| 10 | T1 | Edge break 0.3 on every edge of the bottom face: outline (includes the end faces' bottom edges), slots, reliefs | 1 | 0:12 | |

**Nothing comes loose:**
- The carrier goes as chips, strip by strip.
- The lugs sit on air (op 1 left them 8 thick, open below). Step 4 cuts them away from the free end in, so
  what's left of a lug is always the part still joined to the gauge.
- The bores only split a strip into two pieces that both stay joined inward.
- The side strips are taken from the bar's edge in, each strip still joined along its length.

**Grip and clearance:** the jaws hold the gauge's walls (op 2 Z0 → 4.7); nothing in op 2 goes below Z10.0
(the slot floors), checked on the posted G-code: 5.3 above the jaw tops. rCAD's fixture clearance is 0.8, no flags.

Check results:
- **rCAD simulation:** op 2 starts from what op 1 really leaves. No problems; unfinished 1 mm² (the
  slot/relief corner grid).
- **rcnc simulator:** runs to the end in 846 s, with the lug bores in the bar started 0.4 / 0.3 mm off on
  purpose. Find bores reads both bores 107.000 apart and moves G54 onto them.
  - For the sim only, a Find stock at the start stands in for op 1's G54.

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
