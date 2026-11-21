"""Before op 2's Find bores: stop for the operator to blow the chips out of the two carrier bores (a chip or burr in
a bore shifts X/Y or fails the touch). Run after every re-post."""
import sys
for p in sys.argv[1:]:
    s = open(p).read()
    mark = "(Probe carrier bore A"
    assert "BORES PAUSE" not in s and mark in s, p
    i = s.index(mark)
    block = ("(BORES PAUSE)\nM5\nM9\n(MSG,Blow the chips out of the two carrier bores between the parts, and check for burrs at their mouths; then press Cycle Start)\nM0\n")
    open(p, "w").write(s[:i] + block + s[i:])
    print("added to", p)
