"""Ask rcnc to record op 1a's whole run (beside it as a .wav, with the cut log). Run after posting."""
import sys
p = sys.argv[1]
s = open(p).read()
mark = "(RCNC Z_REFERENCE Vise)\n"
line = "(RCNC RECORD_AUDIO: the calibration passes; rcnc saves the run's sound beside this file as .wav)\n"
assert mark in s and "RECORD_AUDIO" not in s
open(p, "w").write(s.replace(mark, mark + line, 1))
print("added")
