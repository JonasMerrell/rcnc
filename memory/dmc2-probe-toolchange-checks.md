---
name: dmc2-probe-toolchange-checks
description: rcnc PR 25 (2026-11-21): probe tool change does tap / clean-bores / vise-location checks at once; probing then runs straight on
metadata:
  type: project
  modified: 2026-11-21T20:53:33.334Z
---
- Jonas (2026-11-21): wants all probe checks at the T99 tool change (tap the stylus, clean out bores, vise location accurate when VISE= is used), then all probing cycles run with no more stops.
- rcnc PR 25 (MERGED 2026-11-21 as main 9bca2b1; BUILT on the mill PC 2026-11-21 4 PM, title "9bca2b1 +dirty" (dirty = jobs tree)): tool change scans to the next M6 (Interpreter::probe_segment); Continue locked until done. Find stock VISE= + vise ticked = hands off; non-VISE Find stock still stops for the jog. M0 right after a (MSG,...) mentioning "bore" before a FIND_BORES is skipped, so add_bores_pause.py / add_op2_msg.py stops become redundant (they still work without the checks).
- Checks are cleared on Stop / program end / reload; a skipped tool change (probe already loaded) means old behaviour.
- Related: [[dmc2-find-vise-location]], [[dmc2-touch-probe]], [[dmc2-manual-toolchange]]
