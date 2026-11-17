"""Put the stock this job is planned for at the top of a program (so an old file can't pass for a new one)."""
import sys
p = sys.argv[1]
s = open(p).read()
line = "(STOCK 32 x 32 x 120.7 - 6061 bar as measured 2026-11-17; NOT for a 25.4 bar)\n"
if "(STOCK 32 x 32" not in s:
    first = s.index("\n") + 1
    s = s[:first] + line + s[first:]
open(p, "w").write(s)
print("marked")
