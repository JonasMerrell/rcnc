# Crash dumps on the mill PC

If rcnc dies without a message (no panic in `log.txt`), Windows only says so in the Event Viewer
(Application Error 1000) and in a WER report. To have Windows keep a dump that shows what crashed and where:

1. Make the folder `C:\rcnc-dumps`.
2. Import `rcnc-localdumps.reg` (double-click it, as an administrator). It sets
   `HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\cnc-controller.exe` to keep the newest 5
   minidumps there.
3. After a crash, copy the newest `cnc-controller.exe.<pid>.dmp` to a `logs/*` branch along with `log.txt`.

The same in PowerShell (as an administrator):

```powershell
New-Item -ItemType Directory -Force C:\rcnc-dumps
$k = 'HKLM:\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\cnc-controller.exe'
New-Item -Force $k | Out-Null
Set-ItemProperty $k DumpFolder 'C:\rcnc-dumps'
Set-ItemProperty $k DumpType 1 -Type DWord
Set-ItemProperty $k DumpCount 5 -Type DWord
```

A minidump has every thread's stack and the list of loaded modules, which names the DLL the crash was in (a WER
report's "unknown module" usually is a shell extension or driver DLL loaded into rcnc).
