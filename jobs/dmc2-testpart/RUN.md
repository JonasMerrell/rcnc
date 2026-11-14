# DMC2 test part: how to run it

The part is a 6061 block, 35 × 72 × 16 mm, made from your 39 × 77 × 20 stock in two vise setups. rcnc does the
probing, the tool measuring and the work zero; you load tools, jog the probe over the stock once per setup,
and press buttons. Nothing here has run on the real machine yet, so the first pass is an **air cut**.

Files (netdash `~/dmc2/job`): `op1-top.nc`, `op2-bottom.nc`, setup sheets `op1-top.html` and
`op2-bottom.html` (rcnc shows them on its Setup sheet tab when the program
is opened from the same folder). The rCAD document is `dmc2-testpart.json` (open it in rCAD and use Send to rcnc, or copy the
.nc files to the machine PC).

## Before you start

1. **Measure the stock.** Find stock stops (red, nothing set) if the block is more than 1 mm off 39 × 77.
   If yours is different, tell me the sizes and I'll re-post the job.
2. **Spindle:** the job runs every cutter at S24000, your spindle's top speed. Check the VFD gets there.
3. **Coolant:** the job turns flood on (M8) with every cutter. Have it ready, or turn the pump off for the air cut.
4. **Tools.** The program lists its tools; rcnc adds them to its tool table when it is opened (the log
   says which), so each tool change shows the tool's drawing, size and stick-out. Put each in the ER16 with at least this much sticking out of the nut:
   | T | Tool | Collet | Stick-out |
   |---|---|---|---|
   | 99 | YIDOFENG touch probe, Ø2 ball | 6 mm | as it comes |
   | 24 | 3 FLUTE DLC 1/4 (Ø6.35, 3 fl) | 1/4" | 24 mm |
   | 20 | 1/8 ALUMINUM (Ø3.175, 4 fl) | 1/8" | 15 mm |
   | 1 | ALUMINUM chamfer mill, 90° | 1/4" | 22 mm |
   rcnc measures every tool on the tool setter when it goes in, so lengths don't need typing in.
5. **Vise: left station** (the 40 mm gap).
   - Op 1: stock 39 mm across the jaws (X), 77 mm along them (Y), on parallels about 10.9 mm tall so the
     jaws grip only the **bottom 6 mm**: the stock's top is 14 mm above the jaw tops.

## Build rcnc on the machine PC (once)

1. Close Mach3.
2. In `C:\src\rcnc`:
   ```
   git fetch
   git checkout find-stock
   git pull
   cargo build --release --target-dir C:\src\rcnc-test-target
   ```
3. Start it with the **rcnc** shortcut (not "rcnc (simulator)").

## Every session

1. **Reset**, then **Ref All Home**. Tool changes and Find stock refuse to run until Y and Z are homed.
2. **Probe** tab: check *Probe tool* = 99 and *Stylus diameter* = 2. The *Find stock* settings should read
   rough position ±8, search down 25, above the top 5, sides 3 below the top, tolerance 1.

## Op 1, first as an air cut

1. **Open…** `op1-top.nc`.
2. Tick **Air cut** (next to Cycle Start), leave it at **+25 mm**. An amber *AIR CUT +25 mm* badge shows.
   Everything the program cuts now happens 25 mm higher. Tool changes, tool measuring and Find stock still
   happen for real.
3. **Cycle Start.** Z goes up, the bed comes forward.
4. **T99 screen:** put the probe in, press **Continue**. It measures the probe twice on the setter.
5. **Find stock window** (top right):
   1. Tap the stylus with a finger. *Tap the stylus* gets a tick, and Continue unlocks.
   2. With the window's jog buttons (or the arrow keys and Page Up/Down), jog the ball over the **middle of
      the stock**, 2–10 mm above the top. Being a few mm off the middle is fine. A jog that touches the stock
      stops by itself.
   3. **Continue.** It touches the top, then both X sides, then both Y sides.
   4. If the size check fails, the window goes red and nothing is set: check the stock and press
      *Probe again*, or Stop.
   5. Otherwise the log says *Stock found: X 39.0… × Y 77.0…*, and G54 now has X0 Y0 at the middle and Z0 on
      the top.
6. **T24 screen:** swap to T24, **Continue**. It's measured, then faces, cuts the outline, then the pocket.
7. **T20 screen:** swap, **Continue**. It cuts the slot and the two holes.
8. **T1 screen:** swap, **Continue**. It chamfers the top edges.
9. Watch that everything stays well clear of the stock and the vise.
10. Read the log lines starting with **"Probe: touched"**: they show how far the board ran past each touch.
    If you see *"the board kept moving … rcnc stopped it"*, stop and send me the log.

## Op 1 for real

1. Untick **Air cut**. The badge goes away.
2. **Cycle Start** and go through the same screens. Running Find stock again is fine. Keep a hand near
   **Stop** / E-stop on the first cuts.
3. Cycle time is about 3¼ minutes plus the tool changes.

## Op 2

1. Take the part out and blow it off. Flip it **pocket side down** on parallels in the same left station,
   gripping the **finished 35 mm walls**:
   - the jaw tops must be at least **10.5 mm below the part's top face** (the raw bottom, now facing up),
     so the leftover 39 mm wide band of raw stock sits clear above the jaws;
   - that leaves about 8.5 mm of wall in the jaws;
   - put shims or soft jaws against the finished walls and clamp snug, not crushing.
2. **Open…** `op2-bottom.nc`. Do an air cut first if you like (same steps).
3. **Cycle Start:**
   - **T99**: Find stock again. It touches the leftover raw band, which has the same raw faces op 1 was
     centred on, so both sides line up.
   - **T24**: faces to 16 mm thick and cuts off the band, stopping just past op 1's wall.
   - **T1**: chamfers the bottom edge and engraves **DMC2** (it reads correctly from the front of the machine).
4. Cycle time is about 2½ minutes plus the tool changes.

## If something goes wrong

- **Stop** (or Esc) stops the program; **E-stop** cuts the drives.
- If Find stock says the probe is touching before a move, jog it clear and press Continue again.
- If the spindle won't start with an alarm about the touch probe: the probe (T99) is still loaded. rcnc
  won't spin it.
