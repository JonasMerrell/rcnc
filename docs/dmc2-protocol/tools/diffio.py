"""Show changes in the 0x03 I/O frames (OUT payload and IN status) over a time window.

Usage: python -E diffio.py <capture.pcap> <t_start> <t_end>
Prints each time a byte changes; position bytes Q[4..23] and counters Q[44..59] are ignored.
"""
import sys

from decode import packets, unxor_in, unxor_out

IGNORE_IN = set(range(4, 24)) | set(range(44, 60))


def main():
    path, t0, t1 = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
    prev = {"OUT": None, "IN": None}
    for t, d, raw in packets(path):
        if not t0 <= t <= t1:
            continue
        p = unxor_out(raw) if d == "OUT" else unxor_in(raw)
        if p[63] != 0x03:
            continue
        old = prev[d]
        if old is not None:
            ch = [(i, old[i], p[i]) for i in range(63) if old[i] != p[i] and not (d == "IN" and i in IGNORE_IN)]
            if ch:
                print(f"{t:9.3f} {d:3} " + " ".join(f"[{i}] {a:02x}->{b:02x}" for i, a, b in ch))
        else:
            print(f"{t:9.3f} {d:3} initial {p.hex(' ')}")
        prev[d] = p


if __name__ == "__main__":
    main()
