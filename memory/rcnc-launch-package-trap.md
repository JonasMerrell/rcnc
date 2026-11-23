---
name: rcnc-launch-package-trap
description: Never start rcnc on the mill PC from a Remote Control shell: it inherits the Claude MSIX package and reads/writes stale settings copies
metadata:
  type: project
---
- Found 2026-11-20: rcnc (cnc-controller.exe) started from a Remote Control session runs inside the Claude desktop app's package (Claude_pzs8sxrjxfjjc). Its %APPDATA%\rcnc\machine.json and state.json (written as .tmp + rename = new files) go to %LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\rcnc, and reads see those stale copies (machine.json 11-15, state.json 11-19). log.txt appends hit the real file.
- Symptom: after the 2026-11-20 05:51Z rebuild-launch, the saved vise location (and that night's tools/offsets) seemed lost. The real files were intact (machine.json 01:49:52Z, state.json 02:15:43Z). Fix: close rcnc and start it from Explorer or a shortcut.
- RC shells see the package view too: reading %APPDATA%\rcnc from RC shows the package copies. Use WMI (CIM_DataFile) to see the real files.
- How to apply: after a rebuild, do NOT launch rcnc from RC. Ask Jonas to start it himself (or use a launch outside the package). Related: [[dmc2-find-vise-location]].
