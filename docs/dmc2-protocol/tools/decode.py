"""Decode BL-UsbMach HID traffic from a USBPcap capture (see protocol-notes.md).

Usage:
  python -E decode.py <capture.pcap> [summary|frames] [t_start] [t_end] [--cmd 0x04] [--raw]

Uses tshark to pull interrupt transfers, removes the XOR obfuscation and prints commands.
"""
import collections
import subprocess
import sys

TSHARK = r"C:\Program Files\Wireshark\tshark.exe"

M = bytes.fromhex(
    "21F2A19445DF3F99AABEF6D23A772367C23CEEE962387 1E2D2D89DFDFEAF3D5C"
    "9DB23526881A5C5A278381F92CAEDFFAFCCE2633DCCCAAF4D5A9862EC170AEFE"
    "A7".replace(" ", "")
)
assert len(M) == 65


def unxor_out(b):
    return bytes(b[i] ^ M[i] for i in range(64))


def unxor_in(b):
    return bytes(b[i] ^ M[64 - i] for i in range(64))


def packets(path):
    out = subprocess.run(
        [TSHARK, "-r", path, "-Y", "usb.transfer_type == 0x01 && usb.data_len == 64",
         "-T", "fields", "-e", "frame.time_relative", "-e", "usb.endpoint_address", "-e", "usb.capdata", "-e", "usbhid.data"],
        capture_output=True, text=True, check=True).stdout
    for line in out.splitlines():
        t, ep, cap, hid = (line.split("\t") + ["", ""])[:4]
        data = cap or hid
        ep = int(ep, 16)
        raw = bytes.fromhex(data.replace(":", ""))
        if len(raw) != 64:
            continue
        yield float(t), ("IN" if ep & 0x80 else "OUT"), raw


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    flags = sys.argv[1:]
    path = args[0]
    mode = args[1] if len(args) > 1 else "summary"
    t0 = float(args[2]) if len(args) > 2 else 0.0
    t1 = float(args[3]) if len(args) > 3 else 1e12
    want = None
    if "--cmd" in flags:
        want = int(flags[flags.index("--cmd") + 1], 16)
    raw_mode = "--raw" in flags
    counts = collections.Counter()
    first = {}
    last_cmd = None
    for t, d, raw in packets(path):
        if not (t0 <= t <= t1):
            continue
        p = raw if raw_mode else (unxor_out(raw) if d == "OUT" else unxor_in(raw))
        cmd = p[63]
        key = (d, cmd)
        counts[key] += 1
        first.setdefault(key, t)
        if d == "OUT":
            last_cmd = cmd
        if mode == "frames" and (want is None or (d == "OUT" and cmd == want) or (d == "IN" and last_cmd == want)):
            print(f"{t:10.4f} {d:3} cmd={cmd:02X} {p.hex(' ')}")
    if mode == "summary":
        for (d, cmd), n in sorted(counts.items(), key=lambda kv: first[kv[0]]):
            print(f"{d:3} cmd=0x{cmd:02X} count={n:6} first_t={first[(d, cmd)]:.3f}")


if __name__ == "__main__":
    main()

