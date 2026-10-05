# Roland TD-17 — device notes (derived)

> **What this is.** Our own notes on the Roland **TD-17** V-Drums module, written
> to answer one question: what would it take for Tactus to drive it? These are
> **facts** (addresses, sizes, framing) re-expressed in our words — not a copy of
> Roland's document.
> **Sources:** *TD-17 (TD-17-L) MIDI Implementation*, Version 2.00, Sep. 1. 2022
> (`eng04`) and *TD-17 Data List* (© 2022 Roland Corporation), plus Roland's
> published product specifications. Obtain the PDFs yourself —
> see [vendor/README.md](../vendor/README.md) and
> [ADR-0004](../adr/0004-vendor-docs-not-committed.md).

**Status:** documentation and a parsed address map only. There is **no TD-17
profile yet** and nothing has been confirmed on hardware *by us* — §8 collects
what two projects that do have the module found, and our map agrees with theirs.

---

## 1. Why this module

A drum teacher who is himself blind — the teacher of a maintainer's daughter —
uses a **TD-17KVX2**. That makes the TD-17 the first module besides the V31 with
a real, reachable user, and the first honest test of the central architectural
bet: *a new Roland module should be **data**, not code*
([ADR-0007](../adr/0007-device-profile-abstraction.md)).

The short answer, established below: **the bet holds.** The TD-17 speaks the same
Roland SysEx mechanics the `sysex` crate already implements, and the same
parser that reads the V31's parameter tables reads the TD-17's after five small
generalizations — none of them module-specific.

---

## 2. Which module is in which kit

| | |
|---|---|
| Module | **TD-17** — introduced 2018; the same module ships in the 2022 "Gen 2" kits, which differ in pads and cymbals |
| Kits containing it | TD-17K-L (the TD-17-**L** module), TD-17KV, TD-17KVX, and the Gen 2 TD-17KV2 / **TD-17KVX2** |
| Firmware | 1.00 (2018) → 1.01 → 1.02 → **2.00** (Sep 2022, free update; ships on Gen 2 kits) |
| Kit slots | 100 (70 factory presets) |
| Instruments | 310 |
| Trigger inputs | 20 pad "units" (kick … aux, head/rim/bell) |

**TD-17-L** is the reduced module: same SysEx, but **no Bluetooth**. One document
covers both.

Because the module is the same across those kits, a single device profile would
serve every TD-17 owner — the differences between TD-17KV and TD-17KVX2 are pads
and cymbals, not protocol.

---

## 3. SysEx framing

Identical in shape to the V31; the differences are marked **bold**.

- **Manufacturer ID:** `41H` (Roland).
- **Model ID:** **`00 00 00 4B` — four bytes** (the V31's is the three-byte
  `01 06 01`). Our codec takes the Model ID as a slice and the profile stores it
  as a byte vector, so the length difference needs **no code change**.
- **Device ID:** `10H`–`1FH` (17–32) plus `7FH` broadcast; default `10H`. Set on
  the module under **[SETUP] – [MIDI] – [SYS EX]**.
- **Data Request (RQ1), command `11H`:**
  ```
  F0  41  dev  00 00 00 4B  11  aa bb cc dd  ss ss ss ss  sum  F7
  ```
- **Data Set (DT1), command `12H`:**
  ```
  F0  41  dev  00 00 00 4B  12  aa bb cc dd  <data …>  sum  F7
  ```
- **Address and size:** 4 bytes each, 7 bits per byte — same arithmetic.
- **Checksum:** the same Roland scheme over address + data
  (see [PROTOCOL.md §2](../PROTOCOL.md)).
- **Packet rule:** data over 256 bytes is split into ≤256-byte packets sent about
  **20 ms** apart. (The V31 has the same 256-byte ceiling, which the iOS
  transport already respects.)
- The module transmits **only Identity Reply and DT1** — never RQ1.

### Unsolicited edits

The MIDI implementation chart notes that DT1 is *"transmitted if Transmit Edit
Data is on, or when RQ1 is received."* So the TD-17 has the same
**Transmit Edit Data** switch the V31 has: with it on, turning a knob on the
module pushes a DT1 we can listen for; with it off, we only learn state by
polling. That matches `transmit_edit_data` in our capability flags and the
push behaviour already modelled in `devicesim`.

### Identity Reply

```
F0  7E  dev  06 02  41  4B 03  00 00  ss ss ss ss  F7
             └reply┘ └Roland┘ └family┘ └number┘ └─revision─┘
```

- **Device family code:** `4B 03`; **family number:** `00 00`.
- **Software revision** is a *coded* value, not the version digits:

  | Firmware | Revision bytes |
  |---|---|
  | 1.01 or earlier | `00 00 00 00` |
  | 1.02 | `00 00 00 01` |
  | 2.00 | `00 00 00 02` |

  This is worth noting: the V31 profile's `version_format` is a template over
  the four bytes (`"{0}.{1}.{2}{3}"` → "0.2.10"), which assumes the reply carries
  the version itself. The TD-17 needs a second format — a lookup from revision
  code to a spoken version string. That is a **profile schema addition (data +
  one enum arm), not new protocol logic**, and it is
  exactly the kind of per-device difference
  [ADR-0009](../adr/0009-firmware-compatibility-policy.md) expects: detect,
  announce, never block.

---

## 4. Parameter address map

Parsed into [profiles/maps/roland-td-17-address-map.json](../../profiles/maps/roland-td-17-address-map.json)
by `tools/parse_midi_impl.py --device td-17`.

**Four top-level areas** (the V31 has five — it adds set lists):

| Address | Area | Notes |
|---|---|---|
| `00 00 00 00` | Current | a single byte: **Drum Kit Number**, 0–99 (displayed 1–100) |
| `01 00 00 00` | Setup | click + misc (USB input/output gain) |
| `02 00 00 00` | Trigger | per-input trigger parameters |
| `03 00 00 00` | Kit 1 … 100 | stride `00 02 00 00` |

**Inside a kit:**

| Offset | Block | Count |
|---|---|---|
| `00 00 00` | Kit Common | kit name **12 chars**, sub name 16, volume, … |
| `00 01 00` | Kit MIDI | |
| `00 03 00` | Kit Ambience | switch, room type/size/shape, wall type, mic position, level |
| `00 04 00` | Kit Reverb | |
| `00 05 00` | Kit Master Comp | |
| `00 10 00` | Kit Multi FX | type (0–40) + 32 type-dependent parameters |
| `00 20 00` | Kit Unit Common | ×20 pads |
| `00 40 00` | Kit Unit **Main** | ×20 — instrument layer 1 |
| `00 60 00` | Kit Unit **Sub** | ×20 — instrument layer 2 |
| `01 00 00` | Kit Unit VEdit Main | ×20 |
| `01 20 00` | Kit Unit VEdit Sub | ×20 |

Pad units 1–20 are `KICK HEAD`, `SNARE HEAD/RIM`, `TOM1–3 HEAD/RIM`,
`HI-HAT HEAD/RIM`, `CRASH1/2 HEAD/RIM`, `RIDE HEAD/RIM/BELL`, `AUX HEAD/RIM`.

Two structural notes for anyone writing the profile:

- **Two instrument layers** (Main/Sub) where the V31 has three (A/B/C). The
  `dims` mechanism in the profile schema already expresses this — it is a count
  and a stride, not a special case.
- **Kit name is 12 characters** here versus 16 on the V31 (sub name 16 versus 64).
  Name lengths are exactly the sort of thing the V31 cross-check test caught, so
  a TD-17 profile should get the same test.

### Variant tables

The TD-17 documents three kinds of table that restate a block's offsets depending
on a selection. All three parse into the map's `overlays` / `firmware_variants`:

| Kind | Where | Count |
|---|---|---|
| `MFX Type: …` | Kit Multi FX | **41** — the per-effect meaning of MFX Parameter 1–32 |
| `INSTRUMENT GROUP: …` | Kit Unit VEdit | 7 — kick, snare, cross stick, tom, hi-hat, cymbals, other |
| older firmware | Trigger | 1 — the Pad Type list before v1.02 (39 models instead of 46) |

The per-effect tables are a pleasant surprise: for the V31 the equivalent
per-MFX-type parameter names are **not** in section 3 and remain a deferred item.
The TD-17 documents them inline, so its FX editor could speak real parameter
names from day one.

---

## 5. Connecting to it

This is where the TD-17 differs from the V31 in a way that matters to a user, not
just to the code.

- **There is no MIDI IN jack.** The module has **MIDI OUT only**. DIN MIDI can
  therefore never carry our writes — every RQ1 and DT1 has to travel over **USB**
  (or Bluetooth, which we deliberately do not support; see below).
- **USB COMPUTER jack** (USB-B) carries MIDI and audio. In the common case a
  phone connects with a **plain USB-C→USB-B cable**, not a dongle:
  - **Android** — any phone with USB host support (standard for years); the OS
    exposes it through `android.media.midi`.
  - **iPhone 15+ / iPad with USB-C** — direct cable, no camera adapter. Apple's
    4.5 W accessory limit is irrelevant here: the module runs from its own 9 V
    adaptor and draws no power from the phone.
  - **Lightning iPhone/iPad** — needs Apple's Lightning-to-USB camera adapter.
- **Set USB Driver Mode to GENERIC** ([SETUP] – [USB]; takes effect after a power
  cycle). GENERIC uses the operating system's class-compliant driver and is
  limited to USB MIDI — which is all we need. VENDOR selects Roland's own desktop
  driver (MIDI + USB audio) and is not what a phone wants. The V31 needs the same
  GENERIC setting, so this is one habit, not two.
- **Bluetooth LE MIDI exists but is out of scope.** The module offers Bluetooth
  4.2 with the **GATT (MIDI over BLE)** profile, and it is tempting precisely
  because it needs no adapter. Tactus nonetheless supports **USB only**
  ([ADR-0015](../adr/0015-usb-midi-only.md)): BLE's failure modes — pairing
  screens, permission dialogs, silent mid-session drops — are the hardest kind to
  diagnose without sight, and supporting it means two platform implementations
  plus hardware proof that SysEx survives BLE's fragmentation. Bluetooth
  **audio** (A2DP/SBC) is one-way *into* the module for play-along and is
  unrelated to us; the module cannot output to Bluetooth headphones. Bluetooth
  is absent on the **TD-17-L** in any case.
- Relevant module settings live under **[SETUP] – [MIDI]**: MIDI channel,
  Tx/Rx switch, Thru (USB / Bluetooth), Device ID, Transmit Edit Data.

**So the practical path is USB, and on modern phones it is just a cable.** With no
MIDI IN jack and BLE deferred, a phone reaches the TD-17 exactly one way: the USB
COMPUTER port. On Android and on USB-C iPhones that is a single USB-C→USB-B
cable; only Lightning devices add an adapter in the middle. The remaining cost
for a blind user is the one-time module setting (USB Driver Mode → GENERIC),
which someone has to do through the module's inaccessible screen — the same
chicken-and-egg first-run problem the V31 already has, and a good thing to time
when we sit down with the teacher's kit.

---

## 6. What we do not know yet

To be answered with the module in hand (see
[HARDWARE_TESTING.md](../HARDWARE_TESTING.md)):

1. What is the round-trip latency of an RQ1 → DT1 exchange over USB? (Feeds a
   recorded `TimingProfile`.) *Partly answered by proxy (§8): "a few tens of
   milliseconds"; a 240 ms per-block timeout is enough in practice.*
2. Does the module actually push DT1 on front-panel edits with **Transmit Edit
   Data** on, and at what granularity? *Still open — neither project in §8
   listens for unsolicited DT1.*
3. Which **firmware** is on the teacher's unit (2.00 expected on a Gen 2 kit) and
   does the Identity Reply match the coded revision table above? *The table is
   confirmed by proxy (§8); which code 2.01 and 2.10 report is not.*
4. How do the **user kit slots** (71–100) behave versus the 70 presets?
5. Fragmentation: how does the module split replies larger than 256 bytes in
   practice? *Never exercised by either project in §8 — both keep every request
   under one packet.*

---

## 7. What supporting the TD-17 would take

Ordered by dependency; the estimate is deliberately conservative.

1. **`profiles/roland-td-17.json`** — identity (`41 / 4B 03 / 00 00`), model ID
   `00 00 00 4B`, capabilities (100 kits, 20 pads, 2 layers, MFX), and the
   parameters we surface, each citing a `doc` reference into the parsed map.
   *Data only.*
2. **Firmware `version_format`** — one new variant for coded revisions, plus the
   TD-17's three-entry table. *Small schema + enum change.*
3. **Catalogs** — kit list, instrument list and MFX parameter tables come from the
   Data List PDF. `tools/parse_datalist.py` is written against the *V31's* page
   geometry (column boundaries from header glyphs), so this is the one piece
   that needs real parser work rather than a new data file.
4. **Cross-check test** — the same shape as
   `core/crates/device/tests/map_crosscheck.rs`, pinning every profile parameter
   to the parsed map. Cheap, and it is what caught the V31 name-length bug. A
   second cross-check against V-Drum Explorer's hardware-tested block sizes (§8)
   costs one table and buys most of what a bench session would.
5. **Nothing expected in `sysex`, `engine`, or the apps.** The framing, checksum,
   address arithmetic, 256-byte packet rule, poll/verify loop, and speech layer
   are all module-independent.

The realistic risk is not the protocol — it is that we would be shipping a
profile we cannot test until someone with a TD-17 runs it, which is why the
teacher matters more than the document.

---

## 8. Field notes from other implementations (read 2026-10-05)

We will not have a TD-17 in hand before release. Two open-source editors have,
and both are built the way Tactus is — the same RQ1/DT1 mechanics driven by a
per-module map — so their code and changelogs stand in for a bench session.
Nothing below is copied; these are the facts we checked against our own map
and notes, with where each one comes from.

- **V-Drum Explorer** — Jon Skeet, C#, Apache-2.0,
  [github.com/jskeet/DemoCode/tree/main/Drums](https://github.com/jskeet/DemoCode/tree/main/Drums)
  (read at commit `f885f88`, 2026-09-03). Data-driven: one JSON schema per
  module (TD-07, TD-17 v1 and v2, TD-27, TD-50, TD-50X, AE-01/10), the TD-17
  one developed against the author's own module. Blog series:
  [codeblog.jonskeet.uk/category/v-drums](https://codeblog.jonskeet.uk/category/v-drums/).
- **PulsoKit** — arkus1972, Swift + CoreMIDI on macOS, TD-17 kit editor,
  [github.com/arkus1972/PulsoKit](https://github.com/arkus1972/PulsoKit)
  (read at commit `7e11f56`, 2026-08-31). Developed against a real TD-17; the
  TD-27 is recognised but read-only until its map is confirmed — the same rule
  we propose below. Its changelog (`ZMIANY.md`) records what the hardware
  actually did.

### Our derived map agrees with a hardware-tested schema
Block for block, V-Drum Explorer's TD-17 schema (its author's module, firmware
1.0x through 2) says what
[our parsed map](../../profiles/maps/roland-td-17-address-map.json) says:

| Block | V-Drum Explorer | Our map |
|---|---|---|
| Areas | Current `00`, Setup `01`, Triggers `02`, Kit `03` ×100, gap `00 02 00 00` | same |
| Kit sub-blocks | Common `00 00`, MIDI `01 00`, Ambience `03 00`, Reverb `04 00` (fw 2), Master Comp `05 00` (fw 2), MFX `10 00`, Unit Common `20 00` ×20, Unit Main `40 00` ×20, Unit Sub `60 00` ×20 | same |
| KitCommon | size `0x2B`: name 12, sub-name 16, 3 × volume, XStick switch, HH balance −5..5 | same, 43 bytes |
| KitUnitCommon | size `0x1C` on fw 2, `0x18` on fw 1.x | 28 bytes (fw 2 doc) |
| KitUnitInst | size 9: instrument (4), volume (4), bank (1) at offset 8 | same |
| KitUnitVEdit | size `0x41`: placeholder + 16 × 4-byte params, meaning overlaid by instrument group | same, 65 bytes |
| Current | 1 byte, 0–99; **writing it switches the kit** (`SetCurrentKitAsync`) | same |

Two conclusions. The parser that reads Roland's PDF is producing the truth for
this module, not just for the V31. And a TD-17 on firmware **1.x** has a
different `KitUnitCommon` size (and no Reverb / Master Comp blocks) — the coded
revision in the Identity Reply (`00 00 00 02` = 2.00) is what tells us which
layout we are talking to.

### Encodings are family-wide
- **Volume** is `volume32` in V-Drum Explorer: −601..60, tenths of a dB,
  **−601 = −INF** — exactly the V31's sentinel. Our profile `sentinel` mechanism
  carries over unchanged.
- **Instrument bank** is one byte, `0` PRESET / `1` USER — simpler than the V31's
  bank numbers, but still "number + bank", so `catalog_bank` applies.
- **Identity Reply**: V-Drum Explorer matches family code `0x034B` (bytes
  `4B 03`), family number `00 00`, software revision `0`, `1` or `2` — our table
  in §3, confirmed on hardware by a tool that refuses to talk unless they match.

### Transport behaviour both projects saw
- **Reply latency**: "a few tens of milliseconds" (PulsoKit). PulsoKit times out
  a block after **240 ms**; V-Drum Explorer after 1 s. V-Drum Explorer reads
  containers strictly one at a time with **no delay between requests** and waits
  **40 ms after every write**; a whole TD-17 (every container) takes about three
  minutes, a kit a few seconds.
- **Unknown addresses do not answer** at all — no error message. A timeout is the
  only signal (PulsoKit's probe relies on it). Same as the V31.
- **Request sizes**: V-Drum Explorer caps a request at `0x17F` bytes to avoid
  reassembling a fragmented reply; PulsoKit never asks for more than `0x106`.
  Neither has ever seen a split reply. Our per-block reads stay well under one
  packet, so the 256-byte rule in §3 is a ceiling we do not approach.
- **Active Sensing inside SysEx**: the TD-17 sends `0xFE` (System Real-Time),
  and it can land in the middle of a DT1 frame. PulsoKit's reader took it as data,
  the checksum failed, and a read hung until a 60 s watchdog (fixed in its latest release).
  Our reassembler already drops `0xF8`–`0xFF` inside a frame
  ([reassembly.rs](../../core/crates/sysex/src/reassembly.rs)); worth a cassette
  that interleaves one, so it stays that way.
- **Soft Thru** on the module echoes notes back into the SysEx stream and breaks
  a read in progress (PulsoKit manual, troubleshooting). One more setting to name
  at onboarding: **[SETUP] – [MIDI] – Soft Thru: OFF**.
- **First request after connect**: CoreMIDI needs a moment after the port opens —
  an RQ1 sent at once "can vanish without a reply"; PulsoKit waits 700 ms, and
  900 ms for an Identity Reply. Our identity retry already covers this.
- **Program Change on a kit change**: the TD-17 sends one, like the V31 (PulsoKit
  follows the kit through it). PulsoKit handles **running status** — a second
  Program Change may arrive as a lone data byte. Our hint in
  `handle_midi_input` looks for a `Cn` status byte and would miss that form;
  harmless (the poll still catches up), but a cheap fix for both modules.
- **Device ID**: PulsoKit does not assume `10H`; it learns the ID from the first
  checksum-valid DT1 and then ignores others. A good robustness idea for us.
- **USB driver mode**: V-Drum Explorer (Windows) documents that the module must
  be in **VENDOR** mode to be detected, without saying why. PulsoKit, on macOS
  through CoreMIDI, needs no setting at all and never mentions it, which means
  the factory-default **GENERIC** (the OS class driver) carries SysEx on Apple's
  stack. A phone has no other option, so our §5 stands; the Windows requirement
  is a Windows-driver matter.

### A precedent for our "unverified profile" rule
PulsoKit ships the TD-27 behind one flag, `hasVerifiedMap = false`: the module
is recognised, raw dumps are allowed, **no DT1 leaves the app** and no
structured values are shown ("a write to a guessed address lands somewhere in
someone else's instrument; garbage shown as values is worse than an honest
'don't know'"). Its path to verification is a dump plus a photo of the
module's screen plus one changed value, diffed. That is the rule we should adopt
for a TD-17 profile built without the module: recognise, read, announce;
write only once the map is confirmed on hardware.

### Firmware since the documents
2.00 (Dec 2022) added reverb, kit comp and Roland Cloud packs; 2.01 (May 2023)
and 2.10 (Oct 2024, new pad models, Auto Off default) are listed as fixes and
pad support only. Neither project records a map change after 2.00, and
neither records which revision code 2.01/2.10 report — so a Gen 2 kit updated
today may answer with a code our table does not list. ADR-0009 already says what
to do: announce it, never block.
