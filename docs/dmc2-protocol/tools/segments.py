"""Decode 0x04/0x09 motion frames into segments and correlate with reported positions.

Usage: python -E segments.py <capture.pcap> <t_start> <t_end>
"""
import sys

from decode import packets, unxor_in, unxor_out


def seg_fields(s):
    dirs = s[0] & 0x1F
    end = bool(s[0] & 0x20)
    hi = [(s[0] >> 6) & 3, s[9] & 3, (s[9] >> 2) & 3, (s[9] >> 4) & 3, (s[9] >> 6) & 3]
    mags = [s[1 + i] | (hi[i] << 8) for i in range(5)]
    deltas = [-m if dirs >> i & 1 else m for i, m in enumerate(mags)]
    line = s[6] | s[7] << 8 | s[8] << 16
    return deltas, line, end


def main():
    path, t0, t1 = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
    segs, frames, pos = [], [], []
    for t, d, raw in packets(path):
        if not t0 <= t <= t1:
            continue
        if d == "OUT":
            p = unxor_out(raw)
            if p[63] in (0x04, 0x09):
                n = p[62]
                frames.append((t, p[63], n))
                for k in range(n):
                    segs.append((t,) + seg_fields(p[10 * k:10 * k + 10]))
        else:
            q = unxor_in(raw)
            if q[63] == 0x03:
                xyz = [int.from_bytes(q[4 + 4 * i:8 + 4 * i], "little", signed=True) for i in range(5)]
                free = int.from_bytes(q[32:36], "little")
                line = int.from_bytes(q[28:32], "little")
                pos.append((t, xyz, free, q[1] >> 7, line, q[24:28].hex()))
    print(f"frames={len(frames)} segments={len(segs)} first_frame_t={frames[0][0]:.3f} last_frame_t={frames[-1][0]:.3f}")
    tot = [sum(s[1][i] for s in segs) for i in range(5)]
    print("sum of deltas per axis:", tot)
    print("first 12 segments:")
    for s in segs[:12]:
        print("  ", s[1], "line", s[2], "end" if s[3] else "")
    print("last 8 segments:")
    for s in segs[-8:]:
        print("  ", s[1], "line", s[2], "end" if s[3] else "")
    hist = {}
    for s in segs:
        hist[abs(s[1][0])] = hist.get(abs(s[1][0]), 0) + 1
    print("histogram |dX|:", dict(sorted(hist.items())))
    # Movement window from position reports
    moving = [p for p in pos if p[3] == 0]
    if moving:
        print(f"board not-idle from t={moving[0][0]:.3f} to t={moving[-1][0]:.3f} ({moving[-1][0]-moving[0][0]:.3f}s)")
    prev = None
    print("position reports while moving (t, X, free, idle, Q28..31, Q24..27):")
    for p in pos:
        if prev is not None and (p[1] != prev[1] or p[2] != prev[2] or p[3] != prev[3]):
            print(f"  {p[0]:.3f} X={p[1][0]} Y={p[1][1]} Z={p[1][2]} free={p[2]} idle={p[3]} line={p[4]:#x} q24={p[5]}")
        prev = p


if __name__ == "__main__":
    main()
