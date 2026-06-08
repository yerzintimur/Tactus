# ADR-0010: Device instances vs types; the device is the source of truth

**Status:** Accepted · **Date:** 2026-06-07

## Context
Two questions about relating to the live hardware:
1. A user may connect **two identical modules** (e.g. two V31s). They share a
   Model ID and firmware, so their **Identity Replies are identical** (the reply
   has no serial number). We must still tell the units apart.
2. The user can change kits/sounds/settings **outside our app** — on the hardware
   itself, via another editor, or by installing Roland Cloud expansions. The app
   must never drift from reality.

## Decision

### Device *type* vs device *instance*
- The **`DeviceProfile`** identifies the *type* (V31) — [ADR-0007](0007-device-profile-abstraction.md).
- The **instance** (which physical unit) is keyed by the **transport endpoint
  identity**, assigned by the native layer: CoreMIDI `kMIDIPropertyUniqueID`
  (iOS/macOS, stable across reconnects) / Android `MidiDeviceInfo` id. **Not** the
  profile, **not** the Identity Reply.
- Support **multiple concurrent `Session`s**, one per endpoint. Each binds:
  endpoint id + resolved profile + **Roland Device ID** (read from the Identity
  Reply header) + firmware + a **user-assignable label** ("Studio", "Live")
  persisted by endpoint uniqueID. The UI disambiguates by label, falling back to
  endpoint name / Device ID.
- Recommend the user set **distinct Roland Device IDs** (0x10–0x1F) on multiple
  modules — Roland's native disambiguation, and it avoids addressing collisions if
  MIDI streams are ever merged.

### The device is the single source of truth
- We keep **no authoritative copy**. The app holds a cache only for
  responsiveness, always reconciled with the device:
  - active kit → **poll `Current`**;
  - hardware edits → **pushed** via Transmit Edit Data;
  - focusing a kit/parameter → **fresh RQ1 read**.
  Never assume the cache is current. Out-of-band changes are handled by design.
- **Startup loads the minimum:** only the current state (active kit number/name,
  tempo) to announce it. **No full 200-kit dump** — lazy-load the rest on demand.

### Instruments
- The instrument **number** is device state (read from the module). The
  **number → name** mapping is **our catalog**, which can lag Roland Cloud
  expansions. Unknown number → graceful **"Instrument #N (unknown)"**; the catalog
  is versioned/updatable (profile packs).
- **Verify on HW:** whether the module exposes instrument **names** over SysEx. If
  it does, read names live and expansions "just work"; Roland modules usually
  expose only numbers, hence the catalog.

## Consequences
- Native transport exposes a **stable endpoint id + display name** and supports
  **multiple endpoints** (task #13).
- The app keeps a **session registry** keyed by endpoint id with **persisted
  labels** and UI disambiguation (task #20).
- `DeviceInfo` carries the Roland **Device ID**; the instance id + user label live
  app-side (keyed by endpoint uniqueID).
- HW validation checks instrument-name exposure and Device-ID read (task #14).
- Reinforces **no blind writes** (we always present the device's truth) and the
  read-through/lazy model.
