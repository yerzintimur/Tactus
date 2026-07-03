# V31 MIDI / SysEx — protocol notes (derived)

> **What this is.** Our own, independently authored notes on the V31's MIDI
> implementation, written to drive the Rust core. These are **facts**
> (addresses, formulas, ranges) re-expressed in our words and format — not a copy
> of Roland's document. **Source:** *Roland V31 MIDI Implementation, v2.00, dated
> Nov. 11, 2025* and *V31 Data List* (© Roland Corporation; obtain them yourself,
> see [vendor/README.md](vendor/README.md)). Cite that source wherever this data
> is used.

---

## 1. Message framing

- **Manufacturer ID (Roland):** `41H`.
- **Model ID (V31):** `01 06 01`.
- **Device ID:** default `10H` (range `10H–1FH`, plus `7FH` broadcast). Set on the
  module under SYSTEM → MIDI → BASIC → Device ID.
- **Data Request (RQ1), command `11H`:**
  ```
  F0  41  dev  01 06 01  11  aa bb cc dd  ss ss ss ss  sum  F7
  └F0┘ └ID┘└dev┘└model─┘ └RQ1┘└─address─┘ └──size───┘ └sum┘└EOX┘
  ```
  Reply comes back as a DT1.
- **Data Set (DT1), command `12H`:** (write to module, or module's reply)
  ```
  F0  41  dev  01 06 01  12  aa bb cc dd  <data …>  sum  F7
  ```
- **Address:** always **4 bytes, 7 bits each** (top bit 0).
- **Size (in RQ1):** 4 bytes, 7 bits each.

Transmitted by the module: **Identity Reply** and **DT1** only (it never sends
RQ1).

---

## 2. Checksum

> The checksum covers **address + data** (not the F0/41/dev/model/command/F7).

```
sum       = (Σ address bytes + Σ data bytes)           // plain integer sum
remainder = sum mod 128
checksum  = (128 - remainder) mod 128                  // 0 if remainder == 0
```

Reference implementation (Rust):

```rust
/// Roland checksum over the address+data byte slice (everything between the
/// command byte and the trailing checksum, exclusive of F7).
pub fn roland_checksum(addr_and_data: &[u8]) -> u8 {
    let sum: u32 = addr_and_data.iter().map(|&b| b as u32).sum();
    ((128 - (sum % 128)) % 128) as u8
}
```

---

## 3. Golden vectors (encode these as unit tests)

Straight from the spec's worked examples — exact byte strings the core must
reproduce.

### G1 — DT1 write
*Set EQ for layer A of SNARE HEAD of kit 1 to ON.*
- Address `04 00 52 21` (kit1 base `04 00 00 00` + snare-head layer-A `00 52 00`
  + EQ-switch `00 21`), data `01`.
- Sum = `04+00+52+21+01` = `4+0+82+33+1` = `120` → `128-120 = 8` → checksum `08`.
- **Full message:**
  ```
  F0 41 10 01 06 01 12 04 00 52 21 01 08 F7
  ```

### G2 — RQ1 read
*Request the pad-compressor switch for the snare of kit 1.*
- Address `04 02 11 0D` (kit1 base + snare pad params `02 11 00` + comp-switch
  `00 0D`), size `00 00 00 01`.
- Sum = `04+02+11+0D + 00+00+00+01` = `4+2+17+13+0+0+0+1` = `37` → `128-37 = 91`
  → checksum `5B`.
- **Full message:**
  ```
  F0 41 10 01 06 01 11 04 02 11 0D 00 00 00 01 5B F7
  ```

> Test plan: `build_dt1(addr, data)` → G1 bytes; `build_rq1(addr, size)` → G2
> bytes; and `roland_checksum` returns `08`/`5B` for the two payloads.

---

## 4. Value encodings (the part that bites)

A parameter's bytes are **not** a plain little/big-endian integer. The doc uses
several schemes; the core must encode/decode each:

- **Multi-byte 7-bit value:** `aa bb` → `aa*128 + bb` (e.g. value 2356 = `12 34`).
- **Nibble-packed:** a value split into 4-bit nibbles, one per address byte, high
  nibble first. `0a 0b` → `a*16 + b`. Used for e.g. `KitNum` and tempo (see §5).
  The doc marks packet-split fields with `#`.
- **Signed (1 byte):** `00H = -64`, `40H = ±0`, `7FH = +63` (subtract 64).
- **Signed (2 byte):** `00 00 = -8192`, `40 00 = ±0`, `7F 7F = +8191`
  (value = `aa*128 + bb - 64*128`).
- **ASCII:** one char per byte, `0–127` (names).

The core stores, per parameter: `address`, `byte_len`, `encoding`, `range`, and
a `format(value) → String` for TTS.

---

## 5. Key addresses for MVP

### Top-level areas
| Area | Start address | Notes |
|------|---------------|-------|
| Current | `00 00 00 00` | active-kit pointer |
| Setup | `01 00 00 00` | global settings |
| Trigger 1..16 | `02 00 00 00` step `00 01 00 00` | per-pad trigger banks |
| SetList 1..32 | `03 00 00 00` step `00 00 10 00` | set lists |
| Kit 1..200 | `04 00 00 00` step `00 04 00 00` | kit 200 = `0A 1C 00 00` |

### Current (`00 00 00 00`)
- `KitNum` at offset `00 03`, **nibble-packed across offsets `00 00`–`00 03`**,
  total size `00 00 00 04`. Stored value **0–199**, displayed **1–200**.
- **This is the reliable "which kit is active" source — poll it.**

### Kit → KitCommon (offset `00 00 00` within a kit)
- **Kit Name:** offsets `00 00`–`00 0F`, **16 bytes ASCII** (some chars not shown
  on the module display).
- **Kit Sub Name:** offsets `00 10`–… (ASCII).
- **Kit Tempo:** offset **`00 6C`** (4-byte nibble field, `6C`–`6F`), value
  **200–2600** = **20.0–260.0 BPM** (divide by 10). **Confirmed on hardware
  (2026-06-15):** RQ1 to `04 3C 00 6C` (kit 16) returned DT1 `… 00 04 0B 01 …`,
  nibble-decoded = 1201 = 120.1 BPM. *(Supersedes the starter-doc guesses of
  `00 6D` / `00 6F`; the profile's `00 6C` is correct.)*
  **Open semantic question:** the V31 exposes **no on-screen "kit tempo"** the
  user can find (only the metronome, which the user reports as separate). This
  address round-trips and persists, but its user-facing effect (tempo-sync FX /
  click / pattern?) is unconfirmed — investigate before building a tempo UX claim.
- **Kit Tempo Switch:** offset `00 70` (0/1).

### Pad layout within a kit
- `KitUnitCommon` / `KitUnitLayer` index 1–28: KICK(1), SNARE HEAD(2), SNARE
  RIM(3), TOM1 HEAD/RIM(4/5) … HI-HAT HEAD/RIM(12/13), CRASH1/2, RIDE HEAD/EDGE/
  BELL(18/19/20), AUX1–4 HEAD/RIM(21–28).
- `KitPad` index 1–14 (per-pad, not per-zone): KICK..AUX4.
- `KitFx` index 1–4: BUS-A FX1/2, BUS-B FX1/2.

(Exact offsets within a pad/layer — instrument, pitch, decay, volume, pan, EQ,
comp, sends — come from the address-map tables; capture them in the generated
parameter-map JSON, §13 of SPEC, cross-checked against the Data List.)

---

## 6. Real-time behaviour

- **Program Change on kit change is unreliable** (≤128 programs; depends on the
  PROG CHG mapping). Do **not** use it as the primary signal. Poll `Current`;
  treat an inbound PC as a hint to re-poll immediately.
- **Transmit Edit Data = ON** (module setting): the module **pushes a DT1** for a
  parameter when you edit it on the hardware. Parse address+value → update model
  → announce. This keeps the app in sync with physical knob-turning.
- The module also pushes DT1 in response to RQ1 (normal read), and sends Identity
  Reply to an Identity Request.
- **Identity Reply** identifies the module *and* carries its firmware. **Captured
  live (2026-06-15):**
  `F0 7E 10 06 02 41 01 06 03 00 00 02 01 00 F7` —
  device ID `10`, `41` = Roland, **family `01 06`**, **member `03 00`**,
  **software revision `00 02 01 00`** (4 bytes).
  - **Auto-detect** matches on manufacturer + family + member (`41` / `01 06` /
    `03 00`) — *not* on the Model ID `01 06 01` (that's only for DT1/RQ1 framing).
    The device profile stores both.
  - **Firmware mapping (M1 P2):** the 4 bytes `00 02 01 00` correspond to the
    module's own display **"0.2.10 (0031)"** — i.e. the last two bytes `01 00`
    render as the single component **"10"** (so `[a,b,c,d] → "a.b.(cd)"`, *not*
    `"a.b.c.d"`). The build suffix **"(0031)" is NOT in the Identity Reply** —
    it's internal to the module, unavailable over MIDI. Our `FirmwareVersion::
    display` still shows the raw dotted `"0.2.1.0"`; a `version_format`-aware
    renderer is a follow-up. The profile's `firmware.tested` now lists
    `[0,2,1,0]` (so this unit reads as *Tested*, not *Untested*).
    Policy: [ADR-0009](adr/0009-firmware-compatibility-policy.md).

### Kit navigation — observed behaviour (2026-06-15)
- **Current kit** at `00 00 00 00` (nibble, 4 bytes) is the reliable signal; the
  module answers RQ1 reads of it continuously and reflects edits immediately
  (e.g. kit 17 reads `00 00 01 00` = index 16).
- **Shared-address hazard — `select_kit` ("value unknown", observed live):** the
  kit number lives at `00 00 00 00` — the **same address the poller reads** — so
  verifying an app-initiated kit select with an address-keyed edit read-back is
  racy: a poll RQ1 in flight at click time delivers its **stale reply (old kit)**
  onto the verify slot → spurious mismatch/timeout ("value unknown") *before* the
  real change is announced; intermittent, because it needs the poll to be in
  flight. The engine therefore confirms kit selection via the regular `Current`
  read path, not the edit pipeline: write the DT1, issue a `Current` read, ignore
  stale (unchanged) values while the selection is in flight, announce whatever
  kit the device actually lands on — with a tick-driven timeout so a selection
  the module never performs still fails audibly. Reproduced deterministically in
  [timed_scenarios.rs](../core/crates/e2e/tests/timed_scenarios.rs).
- **BUG — speech flood on hardware kit-scroll:** dialling through kits on the
  module pushes an unsolicited Current change per kit; the engine reads each name
  and speaks it, flooding speech. Fix: **debounce** kit-change announcements (only
  speak the settled kit) via the tick timer.

---

## 7. Persistence — RESOLVED on hardware (2026-06-15)

**A DT1 write to a kit address persists across a power-cycle with no separate
save.** Verified end-to-end on the real V31:

1. Set kit 16 tempo to 120.1 — DT1 to `04 3C 00 6C`, data `00 04 0B 01`.
2. Powered the module **off**, then **on** (a full power-cycle).
3. Reconnected → RQ1 `04 3C 00 6C` read back `00 04 0B 01` = **120.1** (not the
   prior 120.0).

So kit-common edits are stored immediately; **no `SNAPSHOT SAVE` / WRITE command
is required** for them, and the app does **not** need a separate "save" step for
kit-common parameters. This clears **risk #1** (SPEC §14).

*Caveats:* verified for one kit-common parameter (tempo). Spot-check a second
parameter family (e.g. a pad/instrument value) before relying on this
universally. The `SNAPSHOT SAVE` action the hardware exposes is likely for
whole-kit/backup snapshots, not required for individual parameter edits.
