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
- **Kit Tempo:** offset **`00 6F`**, value **200–2600** = **20.0–260.0 BPM**
  (divide by 10). *(Note: the starter doc said offset `00 6D`; per the v2.00
  address map the tempo value is at `00 6F`. Verify on hardware in the PoC.)*
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
- **Identity Reply** identifies the module *and* carries its firmware. For the
  V31 the doc gives:
  `F0 7E dd 06 02 41 01 06 03 00 00 02 00 00 F7` —
  `41` = Roland, **device family code `01 06`**, **family member `03 00`**,
  **software revision `00 02 00 00`** (4 bytes).
  - **Auto-detect** matches on manufacturer + family + member (`41` / `01 06` /
    `03 00`) — *not* on the Model ID `01 06 01` (that's only for DT1/RQ1 framing).
    The device profile stores both.
  - The 4 version bytes are the firmware; the exact mapping to a human version
    string is **verify-on-HW**. Used by the firmware-compatibility policy
    ([ADR-0009](adr/0009-firmware-compatibility-policy.md)).

---

## 7. Open protocol question — persistence

The address map has a **temporary/edit view** (`Current`) and the **stored kits**
(`04 …`). The doc shows **no explicit "store/WRITE" SysEx command**, yet the
hardware exposes a `SNAPSHOT SAVE` action. So it is **unverified** whether a DT1
write to a kit address persists across power-cycle or needs a separate save.
**Resolve this on real hardware in the PoC** before the app claims anything is
"saved" (SPEC §14, risk #1).
