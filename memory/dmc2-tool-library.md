---
name: dmc2-tool-library
description: New DMC2 tool library built from scratch 2026-11-21 (geometry only, no stick-out); file location, conventions, open items
metadata:
  type: project
  modified: 2026-11-21T20:23:48.870Z
---
- 2026-11-21 Jonas restarted the tool library FROM SCRATCH (not the Fusion library). Master file: project files tools/dmc2-tools.csv (Fusion-style CSV, rCAD import_tool_library reads it). Old Fusion geometry dump: tools/fusion-geometry-review.csv (reference only).
- Rules (Jonas): transfer GEOMETRY ONLY, no cutting settings (feeds blank; rCAD suggests them). Stick-out is set PER JOB (bodyLength blank). Everything is carbide unless he says otherwise.
- Contents (22, plus probe): existing T1 chamfer, T11 6 mm ball, T24 1/4 3F DLC; balls 2/3/4 mm on 4 mm shanks; 4 mm 45 deg dovetail (rCAD has no dovetail shape, treated as 1 mm slot; separate thread "Add dovetail cutter support to rCAD"); 1/4 bull nose R0.04 in; 1/8 4F; 1/4 4F; 1/4 3F DLC long (25.4 flute, 63.5 OAL); 1 mm 4F (3 mm flute, 4 mm shank); NFC single-profile 60 deg thread mills d2.35/3.15/4.0/4.8; T-slots 6x1 (neck 2.5x8) and 6x2 (neck 2.8x11); SpeTool tapered balls W01001-4 (R0.25/0.5/0.75/1.0 at 5.12/4.16/3.2/2.24 deg per side, 1/8 shank, 15 flute).
- rCAD tapered ball "diameter" = shank diameter (3.175), not Fusion's DC.
- Guesses to confirm: 2 mm ball flute 4 mm, 3 mm ball flute 8 mm, 6x2 T-slot neck dia 2.8 and shank 6.
- rCAD PR 169 (MERGED 2026-11-21): CSV reads neck (tool_shoulderDiameter) and tool_numberOfTeeth (single-profile thread mill otherwise planned one lap only).
- T numbers (auto-assigned, Jonas OK 2026-11-21): T1 chamfer, T2 1mm, T3 1/8, T4 1/4 4F, T5 1/4 DLC long, T6 bull, T11-14 balls 6/2/3/4 mm, T21 6x1 T-slot, T22 6x2 T-slot, T23 dovetail, T24 1/4 DLC, T31-34 thread mills 2.35/3.15/4.0/4.8, T41-44 tapered balls R0.25-R1.0, T99 probe. Guidelines Tools table updated. netdash copy ~/dmc2/tools/dmc2-tools.csv (old fusion-library.csv kept for old jobs); copied and import-checked on netdash 2026-11-22 (22/22). netdash ~/src/cad-rust-app main checkout is on branch gauge-build with uncommitted agent_cam/agent_schema/cam_sim changes, so its rcad binary predates PR 169/170 until someone commits/stashes and rebuilds master.
