---
name: dmc2-wash-down
description: rcnc coolant wash-down feature (PR 23, 2026-11-20): button + (RCNC WASH_DOWN) comment, defaults, open questions
metadata:
  type: project
  modified: 2026-11-20T05:12:27.083Z
---
- rcnc PR 23 (MERGED 2026-11-20; branch claude/project-thread-obbmyv, head 2a1ee0b): Wash down button (main screen + Tools page settings) and `(RCNC WASH_DOWN [PASSES=n])` program comment. Spindle off, Z up to machine Z0, flood on, 2 s soak, lap + 30 mm rows at 3000 mm/min, 2 passes (2nd crosswise), flood off, back to start; a program resumes with spindle/coolant restored.
- Area defaults to saved Find vise location outline (317.7 x 144.5) + 15 mm, clamped to travel; can be set by hand. Settings `wash_down` in machine.json.
- Unconfirmed with Jonas: flood is assumed wired to rcnc's flood output (BlueSkyT OUTPUT1+2, from the Mach3 profile); nozzle assumed on the spindle head. Not tested on the mill; mill PC rebuilt 2026-11-20 to mill/wash-rpm 2632e3e (main + PR 23 + PR 24); untested on the real pump. PR 24 merged 2026-11-21 (main 745d257): mill PC can return to main. Related: [[dmc2-find-vise-location]].
