---
name: dmc2-manual-toolchange
description: State of the DMC2 manual tool change work in rcnc (PR #9) and where Jonas's old Mach3 scripts live
metadata:
  type: project
  modified: 2026-11-14T19:56:19.445Z
---
- rcnc PR #9 (branch manual-toolchange-ux, opened 2026-11-14): M6 = spindle off, Z up first, bed all the way back; big next-tool picture (photo or drawn from tool table, tools/T<n>.png) + big Continue; measures twice on BTER setter, pauses red if readings differ > 0.02 mm (configurable). Not yet run on the real board.
- Positive Y moves the bed back (Jonas, 2026-11-14); Y homes at +512. Tool-change position: Y = setter Y (G53 Y289), NOT the +Y end (Jonas corrected 2026-11-14 after first real run); X at centre of travel.
- Jonas's old Mach3 had two Z buttons after tool setting: set Z to table height and to vise height. Offsets NOT on netdash; the Mach3 folder is on the machine PC (Desktop\Mach3_nw: profile macros, .set screen, HiddenScript.m1s). Setter: G53 X-5 Y289, CalibratedOffset 13.860 - 0.55 = 13.31.
- Found 2026-11-14 in Mach3 screen PA Interface_v05.set: Set Z table CalibratedOffset 63.805 (18.96+49.99-5.145), vise 40.610 (vise top 23.195 above table). Tip on setter top reads Z = offset. Being added to PR #9.
- Related: [[rcnc]]
- X homes at -8 and travels +X about 343 mm (Jonas confirmed X+ moves away from home, 2026-11-14); tool-change X centre = 163.5. Mach3 soft limits (-300..200) are NOT the real travel.
- First real run 2026-11-14: Stop then restart drove Z up into the +Z limit/E-stop. Being fixed (all retracts in G53, modal reset on Stop, Z clamp).
