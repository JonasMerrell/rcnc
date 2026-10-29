# BL-UsbMach (VID 0x3698 / PID 0x0107) — Mach3 plugin protocol notes

> **Capture results (2026-10-28, `captures/`, decoder `tools/decode.py`, `segments.py`, `diffio.py`)**
> These were confirmed on the real board. Where they disagree with the static sections below, this box wins.
>
> - **Board identity:** fw = **325** (0x145), so XOR scrambling is on. Board type Q[59..62] = **0**, i.e. the
>   25–100 kHz variant with a 410-step/segment limit. Date-like code Q[4..7] = 20211223. Chip ID Q[8..19] =
>   `52 FF 73 06 86 78 57 48 18 58 22 67`.
> - **Scrambling:** the XOR tables are exactly as in §1.1. The 0x62 handshake shows as 0x9C if de-XORed, because its
>   last three wire bytes are the plain signature `1F BA 62`.
> - **In-place XOR quirk:** commands that don't rebuild the payload (0x63, 0x0C, 0x64) resend the previous frame's
>   already-XORed buffer, so their payload is junk. Ignore it.
> - **Poll rate:** one 0x03 exchange every ~110 ms (≈ 9 Hz). The board answers within 2–4 ms.
> - **Every Mach3 Reset toggle** runs the full 0x63, 0x62, 0x02, 0x0E, 0x0F, 0x10 sequence.
> - **Motion segment period = 4 ms (250 segments/s).** A G1 X-10 F300 move was 502 segments carrying a total
>   of 2000 steps, mostly 4 steps each (1000 steps/s). That's 83 × 0x04 frames plus 1 × 0x09; the final segment is all-zero
>   with the end flag. The board then took ≈ 2.0 s to execute it.
> - **Direction bit is inverted from my static guess: bit = 1 means positive.** The −X move had all direction bits 0.
> - **Board queue capacity is 250 segments (1 s).** Q[32..35] = free slots: 250 when idle, down to 143 during the
>   move. Q[1] bit 7 = 0 while executing, 1 when idle.
> - **Line ID was 0 for MDI.** Q[28..31] is *not* a line echo: it was 0x000191FF before homing and 0 after.
>   Q[24..27] = 0x0000C297 throughout. Both are unknown.
> - **Status layout confirmed:**
>   - Q[0] = pins 10–17. The probe on pin 11 shows as bit 1, going 1 → 0 when touched (active-low).
>   - Q[0] = 0x6B and Q[41] = 0xCF at idle.
>   - Q[4..23] = X, Y, Z, A, B as i32 steps.
>   - Q[40] = 0x3E (no pendant).
> - **0x0B jog:** axis, dir, 9 999 999 steps, 9331 steps/s (35 % jog × 26660), accel 400 000. It's stopped by 0x0C.
> - **Homing, verified.** Order Z, Y, X. For each axis:
>   1. Seek: 999 999 steps at RefSpeed 20 % (Z 4000, Y 5332, X 5332 steps/s), stopping on the home pin.
>      Z uses pin 15 and Y pin 13, both level 0 (active-low); X uses pin 12, level 1.
>   2. Back off: the opposite direction at 1/10 speed, stopping on the opposite level.
>   3. Clear: 200 steps (1 mm), no pin.
>   4. 0x02 sets the reference value: Z = 1600, Y = 102 400, X = −1600 steps.
>
>   Z and Y seek in the + direction, X in −.
> - **0x0E/0x0F/0x10 values** match the DMC2 profile exactly:
>   - steps/unit 200, 200, 200, 640, 640
>   - Vmax 26660, 26660, 20000, 16000, 16000 steps/s
>   - accel 400000, 400000, 400000, 76800, 76800
>   - output pins: Enable1 → 14, Output#1–#4 → pins 11–14
>   - output enable mask 0x382, invert mask 0x80
>   - soft limits ±steps exactly as in the profile
>   - home-direction bits 0x06
>   - home speed 20 %
> - **0x03 OUT, confirmed fields:**
>   - P[0..3] = output bitmask indexed by Mach3 output-signal number. The bit is the physical pin level after
>     inversion; for example the active-low Output#1 (spindle, bit 7) reads 1 when off and 0 when on.
>     - bit 1 = Enable1: 1 while out of Reset, 0 in Reset.
>     - bit 7 = Output#1 = spindle on pin 11.
>     - bit 9 = Output#3 = flood/mist on pin 13: 1 with M8.
>   - P[4] = spindle duty % (S3000 → 12, S6000 → 24, with max RPM 25000).
>   - **P[5] = spindle-on flag** (1 after M3, 0 after M5), not "hold".
>   - P[8] = 1 in Reset.
>   - P[15..16] = current feed rate (300 after F300).
>   - P[17..18] = max RPM 25000.
>   - **P[24..25] = commanded RPM** (3000 / 6000).
>   - P[28..29] = P[30..31] × duty / 100 (1200 / 2400).
>   - P[30..31] = 10000.
> - **Probe G31 under motion was not captured** (skipped). The probe input bit is confirmed.

Source: static analysis of `BL-UsbMach-100-V325.dll` (SHA256 `FB308109…FDDD79`; the vendor DLL and disassembly
dumps are not redistributed here). 32-bit MSVC DLL, image base 0x10000000. Capture decoders are in `tools/`
(`decode.py`, `diffio.py`, `segments.py`), USB captures of Mach3 driving the board in `captures/`.

Confidence key: **H** = read directly from code, **M** = strong inference from how the value is used,
**L** = guess / needs the USB capture to confirm.

---

## 1. Transport (H)

- Plain HID, Windows inbox driver. Device found with SetupDi + `HidD_GetAttributes` (VID/PID match) +
  `HidP_GetCaps`; opened with `CreateFileA`; I/O is **synchronous `WriteFile` / `ReadFile` of 65 bytes**
  (report ID 0x00 + 64 data bytes). No feature reports, no overlapped I/O. Handle kept at `0x1003C1A8`.
- Usage page 0x008B, usage 0x0001, input = output = 64 bytes (from the HID descriptor Windows cached).
- In this document `P[0..63]` = the 64 data bytes of an OUT report, `Q[0..63]` = the 64 data bytes of an IN
  report (i.e. byte 0 of the 65-byte buffer is the report ID and is never touched).
- **`P[63]` is always the command byte.** A reply to a command that expects one echoes it in `Q[63]`.
- Only three commands are followed by a `ReadFile`: **0x03** (I/O + status exchange), **0x62** (handshake),
  and the 0x03 sent during shutdown. All other commands are fire-and-forget.

### 1.1 Payload obfuscation (H)

Fixed 65-byte XOR table `M` at `0x1003B080` (static, never written at runtime):

```
21 F2 A1 94 45 DF 3F 99 AA BE F6 D2 3A 77 23 67  C2 3C EE E9 62 38 71 E2 D2 D8 9D FD FE AF 3D 5C
9D B2 35 26 88 1A 5C 5A 27 83 81 F9 2C AE DF FA  FC CE 26 33 DC CC AA F4 D5 A9 86 2E C1 70 AE FE
A7
```

- OUT (send, `0x10002250`):  `wire[i] = P[i] ^ M[i]`        for i = 0..63
- IN  (recv, `0x100021C0`):  `Q[i]    = wire[i] ^ M[64-i]`   for i = 0..63 (table read backwards, offset by one)
- Applied only when flag `0x1003C3D3` is set. It is set from the firmware version `fw` reported in the handshake:
  **on** when `fw == 0` (not yet known), `222 ≤ fw < 310`, `312 ≤ fw < 320`, `fw ≥ 322`;
  **off** for `1 ≤ fw < 222`, `fw ∈ {310,311}`, `fw ∈ {320,321}`.

```
10002256  cmp  byte ptr [0x1003c3d3], 0        ; encryption enabled?
100022A3  mov  bl, byte ptr [ecx + 0x1003b07f]  ; key[ecx-1]
100022A9  xor  byte ptr [ecx + eax], bl         ; buf[ecx] ^= key  (ecx = 1..64)
...
100022FD  push 0x41                             ; 65 bytes
10002301  call WriteFile
```

---

## 2. Command summary

| P[63] | Name (my label) | Direction / reply | Sent from | Conf. |
|---|---|---|---|---|
| 0x62 `'b'` | Handshake + input/pin config | OUT, then IN reply with `Q[63]=0x62` | `Reset` → `0x100075C0` | H |
| 0x03 | I/O + status exchange (poll) | OUT, then IN status report | every `Update` (`0x10003950`), Reset, shutdown | H |
| 0x02 | Set axis position counters | OUT only | after homing, after connect (`0x10008C50`) | H |
| 0x04 | Motion block: 6 trajectory segments | OUT only | `0x100079C0` (from `Update`) | H |
| 0x09 | Motion flush / end-of-move block (≤6 segments, last flagged) | OUT only | `0x10002DB0` when Mach3 queue empties | H |
| 0x0B | Single-axis move / jog / home-seek / probe (stop-on-pin) | OUT only | `JogOn`, `Home`, `Probe`, homing state machine | H |
| 0x0C | Jog stop (decelerate) | OUT only | `JogOff` → `0x10004EE0` | H |
| 0x0E | Config 1: steps/unit, max vel, accel per axis | OUT only | connect (`0x100062D0`) | H (fields M) |
| 0x0F | Config 2: step/dir/output pin map | OUT only | connect (`0x100062D0`) | H (fields M) |
| 0x10 | Config 3: soft limits, home dir, home speed | OUT only | connect (`0x100062D0`) | H (fields M) |
| 0x61 `'a'` | Save plugin settings (5 × float32) | OUT only | Config dialog OK (`0x10001000`) | H (meaning L) |
| 0x63 `'c'` | Stop / abort / flush queue (payload zero or don't-care) | OUT only | Reset, limit hit, plugin unload, E-stop | H |
| 0x64 `'d'` | Disconnect / goodbye | OUT only | Mach3 window destroy (`0x10008D20`) | H |

Board-side unsolicited data: none seen — the board only answers 0x03 and 0x62.

---

## 3. Connect / handshake sequence (`Reset` = `0x100056F0`, H)

1. `Sleep(30)`; if a previous session was up, send **0x63** (abort).
2. Build handshake payload (below) and send **0x62**, `ReadFile` reply, require `Q[63] == 0x62`.
   Failure → "No UsbMachBoard, Please plug it in!".
3. "UsbMach connected"; `Sleep(30)`.
4. **0x02** set-position with Mach3's current motor positions.
5. `Sleep(30)`; **0x0E**, **0x0F**, **0x10** config frames.
6. Build I/O block, send **0x03**, read status.

### 3.1 0x62 handshake OUT payload

| Offset | Content | Conf. |
|---|---|---|
| P[0..52] | One byte per Mach3 input signal (Mach3 order: X++,X--,XHome,Y++,…,Probe=22,Index,LimitOvrd,EStop=25,…,OEM Trig…). Value = `(pin << 1) \| activeLow` if the signal is enabled and on pins 10..25 (10..26 for fw ≥ 320), **pin 14 excluded**; else 0 | H |
| P[24] | Overridden with the first enabled axis-home signal when LED 0x22 is on | M |
| P[54] | bit n = motor n flag at Engine+0x28073 (stride 0x60) — likely *motor enabled* | M/L |
| P[55] | bit n = motor n flag at Engine+0x2805D — likely *step active-low* | L |
| P[56] | bit n = motor n flag at Engine+0x2805C — likely *dir active-low* | L |
| P[57] | bit n = flag at Engine+0x286D8 (stride 0x68, 7 entries) | L |
| P[58], P[59] | enable mask / invert mask for 5 output signals (Engine+0x28645…) | L |
| P[60] | 1 when called with Reset(9) | M |
| P[61..63] | **Wire bytes always `1F BA 62`** (signature + command) | H |

Quirk: while `fw` is still 0 (first connect) the plugin pre-XORs P[0..59] with `M`, so after the send-side XOR
those bytes go out **in clear**; P[41] and P[51] are then forced to 0x35 / 0x8E (pre-XOR). Expect the
handshake to look partly plaintext in the capture.

### 3.2 0x62 reply (IN, after de-XOR)

| Offset | Content | Conf. |
|---|---|---|
| Q[63] | 0x62 echo (required) | H |
| Q[8..19] | 12 bytes shown as hex — looks like an MCU 96-bit unique ID | M |
| Q[10], Q[16..19] | Printed with `%x` into a second ID string (serial/model) | M |
| Q[4..7] | u32 printed `%d` (OEM/user code?) | L |
| Q[20..23] | u32 **firmware version** (0..999; e.g. 325 ⇒ "V3.25"). Drives the encryption and feature switches | H |
| Q[59..62] | u32 **board type**. `0x00062F3E` ⇒ fast board: step-rate choices 50/100/140/200 kHz (default 140), max 900 steps per segment. Anything else ⇒ 25/45/65/100 kHz (default 65), max 410 steps per segment | H |

The chosen kHz × 1000 is written into Mach3's per-axis max step rate (`Setup+0x246198…`, 7 doubles). The kHz
choice is persisted by the plugin to `\MACH3\plugin.dat` (drive root; does not exist on this PC ⇒ defaults).

---

## 4. 0x03 — I/O + status exchange (H)

Sent once per Mach3 `Update` call (and during Reset/shutdown); always followed by `ReadFile`.

### 4.1 OUT payload (I/O block built by `0x100042C0`, stored at `0x1003C230`)

| Offset | Type | Content | Conf. |
|---|---|---|---|
| P[0..3] | u32 | Output state bits — one bit per Mach3 output signal that is mapped to the board (30 signals, 5 per group × 6), bit = logical active state | M |
| P[4] | u8 | Spindle PWM duty, **percent** = S × SpindleOverride% / MaxRPM (DROs 817, 74, 57) | M |
| P[5] | u8 | Hold flag (Mach3 LED 11 with 2-sample debounce; forced 0 in E-stop) | L |
| P[6..7] | u16 | Value of `Setup+0x8F4A8` (Z steps-per-unit slot) | L |
| P[8] | u8 | Mach3 reset/E-stop state (Engine+0x289DF; 1 = in reset) | H |
| P[15..16] | u16 | DRO 55 (fw 0x145/0x14E/0x14F only) | L |
| P[17..18] | u16 | DRO 57 = max spindle RPM (same fw only) | M |
| P[19] | u8 | LED 118 | L |
| P[20..23] | f32 | DRO 828 | L |
| P[24..25] | u16 | Spindle PWM in 0.1 % (fw 320–329, ≥ 334) or DRO 202 (fw 0x145/0x14E/0x14F); else Engine+0x28C7F | M |
| P[26] | u8 | Engine+0x28C7F | L |
| P[28..29] | u16 | P[30..31] × duty% / 100 | M |
| P[30..31] | u16 | PWM base from profile × 10, clamped: <20 ⇒ 200, >1000 ⇒ 10000 (DMC2 PWMBase=5000 ⇒ 10000) | M |
| P[32..51] | 5 × f32 | Per-axis doubles from `Setup+0x11DC40…` (axis scale or similar) | L |
| P[52..55] | u32 | Spindle-encoder related (fw 0x145/0x14E/0x14F) | L |
| P[63] | | 0x03 | H |

### 4.2 IN status report (copied to `0x1003C270`, parsed in `0x10003950`)

| Offset | Type | Content | Conf. |
|---|---|---|---|
| Q[0] | u8 | Input pins 10..17 (bit0 = pin 10 … bit7 = pin 17), raw level | H |
| Q[1] | u8 | bit0 = pin 25 (fw ≥ 320: bits 0–1 = pins 25, 26). **bit7 = motion idle / queue empty** (gates jog, home, probe; drives the homing state machine) | H |
| Q[4..23] | 5 × i32 | **Absolute axis positions in steps**, X Y Z A B (written straight into Mach3 motor positions) | H |
| Q[28..31] | u32 | Currently executing **line ID** (24-bit, as sent in the segments); 0xFFFFFF = none | M |
| Q[32..35] | u32 | **Free space in the board's motion queue**. The host sends motion only while ≥ 100; it is set to 200 on reset | H |
| Q[40] | u8 | Flags: bit0 = MPG/pendant active, bits1–5 = pendant axis select (first 0 bit), bit6 = ?, **bit7 = request position resync** | M |
| Q[41] | u8 | Input pins 17..24 (bit0 = pin 17 … bit7 = pin 24) | H |
| Q[42] | u8 | State code; 2 = queued move finished (checked when ending a sequence) | L |
| Q[44..59] | 4 × u32 | Free-running counters; plugin computes axis velocity = Δsteps × 2 400 000 / Δcount (⇒ ~2.4 MHz timebase?) | L |
| Q[60] | u8 | Pendant key code (fw 0x145/0x14F): 2 = start/hold, 0x0B..0x10 = run macros m931/m930/m933/m999/m941/m942, 0x14/0x15 = jog, 0x2n = jog step size | M |
| Q[63] | u8 | Echo 0x03 | H |

Input pin numbering is the board's own (Mach3 port 1, pins 10–25); the plugin maps each enabled Mach3 input
signal on those pins and applies its Neg flag. **Limits are handled on the host**: when a ±limit signal goes
active the plugin sends **0x63**, raises E-stop and reports "X Limit Switch Triggered!" (unless Limit Override).

---

## 5. Motion streaming — 0x04 / 0x09 (H, timebase L)

Mach3 runs the trajectory planner itself; the plugin drains Mach3's 4096-entry trajectory ring
(`Engine+0x28000` head, `+0x28004` tail, entries 0x120 bytes at `Planner+0x11F898`). For each point it
computes the **per-axis step delta since the previous point** and packs it as a 10-byte segment. Per
`Update` it sends at most 36 points, and only while `Q[32..35] ≥ 100`.

Frame (`P[63]` = 0x04 when full, 0x09 for the final/flush frame):

| Offset | Content |
|---|---|
| P[10k .. 10k+9] | Segment k, k = 0..5 |
| P[62] | Number of valid segments (1..6) |
| P[63] | 0x04 or 0x09 |

Segment layout (10 bytes, H):

| Byte | Content |
|---|---|
| 0 | bits 0–4 = direction for X,Y,Z,A,B (1 = negative). bit 5 = **end-of-move** (set on the last segment of a 0x09 frame). bits 6–7 = bits 8–9 of \|ΔX\| |
| 1..5 | bits 0–7 of \|ΔX\|, \|ΔY\|, \|ΔZ\|, \|ΔA\|, \|ΔB\| (steps in this segment) |
| 6..8 | Mach3 line ID, 24-bit LE (saturates at 0xFFFFFF) |
| 9 | bits 8–9 of \|ΔY\| (bits 0–1), \|ΔZ\| (2–3), \|ΔA\| (4–5), \|ΔB\| (6–7) |

- So each axis delta is a 10-bit magnitude + sign; the plugin refuses deltas > 900 (fast board) / 410
  ("Speed is too high").
- Direction bit for an axis with Δ = 0 repeats that axis's last direction (`0x1003C454..58`).
- Flush with no pending segments: P[0] = 0x20, P[62] = 1, P[63] = 0x09.
- **Unknown: the time length of one segment** (Mach3's trajectory point period). The board presumably
  plays segments at a fixed rate and generates step pulses evenly within each. To be measured from the capture
  (segments/s at a known feed rate).
- The board reports the absolute positions back in every 0x03 status (Q[4..23]).

```
10008145  mov  al, byte ptr [ebp-0x18]   ; |dX| >> 8
1000814F  shl  al, 6
10008152  or   al, byte ptr [ebp+0x5b]   ; direction bits
1000815C  mov  byte ptr [0x1003c2b0], al ; segment byte 0
10008175  mov  byte ptr [0x1003c2b9], bl ; byte 9 = hi bits Y/Z/A/B
1000817B  mov  byte ptr [0x1003c2b6], al ; bytes 6..8 = line id
100081AA  mov  byte ptr [0x1003c2ef], 4  ; command 0x04
10008BBD  mov  byte ptr [0x1003c2ee], cl ; segment count -> P[62]
```

---

## 6. 0x0B — single-axis move until pin (H)

Built by `0x100074E0`; all fields little-endian int32:

| Offset | Content |
|---|---|
| P[0..3] | Axis 0..4 (X,Y,Z,A,B) |
| P[4..7] | Direction: 1 = positive, 0 = negative |
| P[8..11] | Distance in steps (magnitude) |
| P[12..15] | Velocity, steps/s |
| P[16..19] | Acceleration, steps/s² |
| P[20..23] | Stop pin (0 = none, else board input pin 10..25) |
| P[24..27] | Stop level: 1 = stop when pin reads active-high / 0 = active-low (Mach3 Neg flag inverted) |
| P[63] | 0x0B |

Uses:
- **Jog** (`JogOn`): stop pin 0. Distance = to the soft limit (or ±9 999 999). Velocity = jog% × max vel × steps/unit (× 0.05 when Limit Override is on). Stopped with **0x0C** (`JogOff`).
- **Home** (`Home` → `0x10005000`), a host-side state machine using Q[1].bit7 (idle, seen twice):
  1. Seek: distance ±999 999 toward HomeNeg direction, speed = RefSpeed% × max vel; stop when the home pin becomes active.
  2. Back off: reverse, speed/10, stop when the pin releases.
  3. Clear: move 1 unit (steps/unit) further with no stop pin.
  4. Verify the switch is released, set the axis to its home reference value, send **0x02**, "home finished".
- **Probe / G31** (`Probe` → `0x100052D0`): **single axis only**. It picks X, Y or Z, whichever has a nonzero target distance (DROs 800–802), at the feed of DRO 818 / 60 × steps/unit, stopping on the probe pin (Engine+0x28451) with its polarity. Probe position = the board's position report after it stops. A latched probe position was not found; to confirm with the capture.

## 7. 0x02 — set positions (H)

The payload is the last status block `Q[0..63]` with `Q[4..23]` replaced by 5 × i32 target positions (steps)
and P[63] = 0x02. Used after homing and right after connect.

## 8. Config frames at connect (`0x100062D0`)

**0x0E** (H layout, M meaning):

| Offset | Content |
|---|---|
| P[0..9] | 5 × i16 steps-per-unit (X..B) |
| P[10..29] | 5 × i32 max velocity (steps/s) |
| P[30..49] | 5 × i32 acceleration (steps/s²) |
| P[50] | A-axis rotary/slave flags |
| P[51] | B-axis rotary/slave flags |
| P[52] | 1 or 2 (Engine+0x28B54 flag) |

**0x0F** (pin map, M):

| Offset | Content |
|---|---|
| P[0..4] | Step pin − 1 per axis (defaults 0,2,4,6,8) |
| P[5..9] | Dir pin − 1 (defaults 1,3,5,7,9) |
| P[10..39] | Output pin − 1 for each mapped output signal (defaults 0x0A–0x0D for P[17..20]) |
| P[40..43] | u32 output enable mask |
| P[44..47] | u32 output invert mask |
| P[48] | Engine+0x28D84 |

**0x10** (limits/home, M):

| Offset | Content |
|---|---|
| P[0..19] | 5 × i32 soft limit (Engine+0x28C4C; probably max, steps) |
| P[20..39] | 5 × i32 soft limit (Engine+0x28C64; probably min, steps) |
| P[40] | Home-direction bits |
| P[41..45] | Home speed % per axis |

## 9. Mach3 plugin interface (H)

- **Exports:** InitControl, PostInitControl, Config, StopPlug (both no-ops), Update, Notify, Reset, JogOn, JogOff, Home, Probe, Purge, DoDwell, SetProName, the SetGet/Set DRO and LED callbacks, SetDoButton, SetCode and SetGetMenuRange.
- **Window hook:** PostInitControl adds a "BL-UsbMach" menu and subclasses the Mach3 window. On `WM_DEVICECHANGE` (arrival/removal) it reconnects. On `WM_DESTROY` it sends **0x63** (zero payload), then **0x03** with all outputs off, then **0x64**, and closes the handle.
- **Update (`0x10002D20`):** if connected, one **0x03** exchange. Then either the motion streamer (`0x10002DB0`) or an alternate path (`0x100038B0`), then rebuild the I/O block and run the home/probe state machine (`0x10004790`). Polling rate = Mach3's Update rate; measure it in the capture.

## 10. Open questions for the capture

1. Segment period (time per trajectory point) and Update/poll rate.
2. Confirm the XOR tables and that fw ≥ 322, i.e. encryption is on, for this board.
3. Firmware version and board-type values in the 0x62 reply.
4. Spindle PWM byte(s) and output bits for M3/M5 and M8/M9 on this machine (Output#1 = pin 11, Output#3 = pin 13 in the DMC2 profile).
5. Meaning of Q[42], Q[44..59], P[5], P[6..7], P[32..51].
6. Probe: does the board latch a trigger position, or is the stop position used?
