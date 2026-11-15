# Phone stand op 2: handoff for the rCAD adaptive work

This folder holds the handoff from the netdash thread that worked on cad-rust-app branch
`adaptive-spindle-power`. That thread stopped work on the branch at **dcc8825**, which is
pushed. Nothing is left uncommitted.

## What is here

| File | What |
|---|---|
| `stand.json` | The rCAD part document: the phone stand, built by `build.py` from `shape.py` (`build.json` is the rcad run script that `build.py` writes). |
| `cam.py` → `cam.json` | Builds both setups. Op 1 (underside, cam 0) comes first. Then op 1 is re-stocked for the restart contour. Op 2 (top, cam 1) inherits op 1's result as its stock: the bores and the plinth are already cut when op 2 starts. Machine limits are in `M` at the top of `cam.py`: 1.5 kW × 0.78, rated 24k, max feed 3000. The tools are in `tools()`. The posted programs come from `rcad run cam.json`. |
| `add_chip_pause.py` | Inserts the CHIP PAUSE block (M0 + MSG) into op2-top.nc after the rough. Re-run it after every re-post. |
| `fusion-library.csv` | The rows of the library `import_tool_library` reads, for this job only: T1 ALUMINUM chamfer, T11 6 mm ball, T24 3 FLUTE DLC 1/4. Same format as `~/dmc2/tools/fusion-library.csv`. Load it with `"material": "carbide"`; the library says HSS, which is wrong. |
| `tools-full-format.csv` | The same three tools in the tab-separated format of `fusion-library-full.csv`. |
| `variants/n_cur.json`, `n_dn12.json`, `n_dn9.json` | Op 2 with the three rough settings. They differ only in the `3D rough (adaptive)` op: <br>• `n_cur`: stepdown 3.3, stepover 0.22 D, leave 0.3/0.2 (the posted settings) <br>• `n_dn12`: stepdown 12, stepover 0.095 D <br>• `n_dn9`: stepdown 9, stepover 0.095 D <br>Each one ends with `simulate_cam` for cam 1 and exports nothing. |
| `variants/rep.py` | Summarises a run's output per op: time, air %, spindle power %, worst load, chatter index, chip fill, warnings. |

T99 (the touch probe) is not in the library. `cam.py` makes it from rCAD's built-in "Touch probe" preset: Ø2 ball, flute length 12.

## Commands

Run these from this folder, with rcad built from `adaptive-spindle-power`:

```sh
cargo build --release -j2            # in cad-rust-app; one build at a time on netdash
R=~/src/cad-rust-app/target/release/rcad
$R run variants/n_cur.json > n_cur.out 2>n_cur.err     # about 10-25 min each; run them one at a time
python3 variants/rep.py n_cur                           # reads n_cur.out
# the posted programs:
python3 cam.py      # writes cam.json (already here)
$R run cam.json     # op1-underside.nc/.html, op1-restart-contour.nc/.html, op2-top.nc/.html
python3 add_chip_pause.py
```

`rcad run` prints one JSON line per tool call. `rep.py` takes these values from the `simulate_cam` result:

| Index | Field in `result` |
|---|---|
| Spindle power % and worst load | `loads[op].spindle_power_pct` and `loads[op].load` |
| Air % | `air[op].air_pct` |
| Chatter index | `chatter[op].margin`; over 1 means chatter is expected |
| Chip fill | `chips[op].chip_fill` |
| Left rough | `unfinished.area_mm2` |

## The baseline (posted op2-top.nc)

The posted op 2 was simulated from its own `cam.json` at rCAD 2fd7b43 (rcnc 4a396dd + 048ce6b + 241be67). It was not imported from the G-code; the numbers come from the same `simulate_cam`:

| Index | Posted program |
|---|---|
| Rough spindle peak | 72% |
| Chatter | 0.82 |
| Chip | 0.72 |
| Air | about 70% |

That program is the one on the machine. Targets for a new adaptive:

- rough air under about 30%;
- spindle peak at or under about 70–72%;
- chatter and chip no worse than 0.82 / 0.72.

## Results so far (rough only)

| Build | Time | Air | Spindle peak | Chatter | Chip |
|---|---|---|---|---|---|
| master #144 as merged | 953 s | 14% | 104% | 1.27 | 0.70 |
| dcc8825, `n_cur` | 1251 s | 21% | 94% | 1.89 | 0.72 |
| dcc8825, deep-narrow 12 / 9 mm | 1469–1672 s | 30–33% | 82–119% | | |

The deep-narrow variants lose because the power cap narrows the stepover to 0.3–0.4 mm on the deep levels, and because of the wall spikes described below.

## Where the logic lives (cad-rust-app `src/cam_3d.rs` at dcc8825)

- **`adaptive()` (about line 2160): the stepover per level.**
  - `area_max`: the tool's cut-area limit.
  - `area_power`: the power cap. It uses a Kienzle kc at the mean chip thickness for the radial width, and `POWER_SHARE` 0.7 of the power available at this rpm.
  - `level_step = step.min(AREA_SHARE*area_max.min(area_power)/deep).min(WIDTH_SHARE*widest(d,deep))` (about line 2228), with `AREA_SHARE` 0.4 and `WIDTH_SHARE` 0.25.
  - When the stepover is narrowed, the op gets a warning.
- **`struct Level` (about line 1670): one Z level of clearing.**
  - `clear()` (1798) and `grow()` (1816) grow the cleared region outward by passes.
  - `open_pass()` (1941) is the pass along the edge of what is left. **The last band against a wall is cut here. It is where a band wider than the stepover gets taken in one go: the "wall band".**
  - `go_to()` (1718) chooses how to get to the next pass:
    - a link at `fast` (machine max 3000) when `link_clear()` (1702) says the link is over cleared centres;
    - otherwise a cut at the feed when the move is short;
    - otherwise `helix_in` inside the material;
    - otherwise an approach from above.
  - `run()` (1745) applies `arc_feed()` (2111): the feed is lowered on tight arcs (R < 0.5 r), so the outer edge stays at the feed. This applies only where the tool reaches material.

## How the wall band showed up

1. Look at the `simulate_cam` result for the `3D rough (adaptive)` op:
   - `loads[op].width`: the radial engagement at the worst sample;
   - `loads[op].at`: where that sample is (part coordinates);
   - `depth` and `feed` at the same sample.
2. Then find the move in op2-top.nc at that X/Y/Z.

At dcc8825, `n_cur` gives:

| Index | Value | Where (X, Y, Z) | Engagement |
|---|---|---|---|
| Worst load (`loads`) | 94% | 11.74, −16.61, 13.2 | width 4.19 × depth 3.1, at F3000 |
| Chatter (`chatter`) | 1.89 | 17.41, −18.46, 13.2 | width 5.30 |
| Chip (`chips`) | 0.72 | −21.21, −16.59, 13.2 | |

The worst load is a straight link `X24.198 Y-16.584 F2400 / X11.741 Y-16.608` at Z13.2 along the backrest foot. `link_clear` passes it because `free` (the generator's cleared-centre set) believes that band is already cleared. The real stock there still has the band.

The same kind of band (about 4.3 mm wide, full stepdown) appears at the lower levels (Z10.2) where the last ring meets the wall.

One debug check returned the same answer at every level from Z26.2 down to Z10.2: the stock column at that point reads `(2.0, 29.5)`, and both `material` and the level area contain the point. The stock model and the area are right there. **What goes wrong is the generator's `free`/cleared bookkeeping, not the stock.**

## Tried and failed (or only half worked)

- **`fast = feed` for links.** Peak 80%, but slower, and the band is still cut at full width.
- **Links limited to 3 r.** Peak 74%, but two cam_3d tests failed (the dome/pocket link tests).
- **Edge-travel correction of the load sample in `cam_sim::load_sample`.** It gave 280 false problems and was reverted.
- **Arc feed compensation everywhere.** Dome test air rose to 32%. It is now limited to R < 0.5 r and to where the tool reaches material.
- **Chatter 1.89.** It is driven by the width measure on tight spiral arcs (per centre travel) and by the spiral out of the op 1 bores (open up to Z14.5). The spiral is fixed in the feed but not in the chatter measure. A per-edge-travel width measure for chatter was not tried.

## Ideas not done yet

1. Make the last wall band at most the stepover: in `open_pass()` (or before it), split a remaining band wider than `level_step` into passes.
2. Make `free` come from the real cleared region (what the passes swept, eroded by r) instead of the planned one, so `link_clear` can't link through an uncut band.
3. Measure chatter width per edge travel on arcs.
