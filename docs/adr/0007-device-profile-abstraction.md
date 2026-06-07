# ADR-0007: Device-profile abstraction for multi-device support

**Status:** Accepted · **Date:** 2026-06-07

## Context
The app started for the Roland V31, but we want to support V51/V71 and **future,
unreleased modules** without re-architecting. The maintainer only has a V31 to
test on. Roland's modules share the same SysEx framework (RQ1/DT1, checksum,
4-byte 7-bit addresses, nibble/signed/ASCII encodings); what differs per module is
the **Model ID**, the **parameter address map**, the **catalogs** (instruments /
FX / ambience), and **capabilities** (pad layout, kit count, FX slots, features).

If we hardcode V31 specifics into the core or the apps, every new module means
invasive code changes — the opposite of "architecturally correct."

## Decision
**Separate the invariant protocol mechanics (code) from the per-device model
(data).**

- The `sysex` crate implements only the Roland-wide mechanics and knows nothing
  about any specific module.
- Everything module-specific is a **`DeviceProfile`** — versioned **data**
  (JSON/RON under `profiles/`): `model_id`, `capabilities`, `areas`/address map,
  `parameters` (offset/len/encoding/range/unit/i18n key), and `catalogs`.
- A `ProfileRegistry` holds embedded profiles and can accept **downloadable
  profile packs** (future modules / catalog expansions without an app update).
- On connect, an **Identity Request** is sent; the Identity Reply's **Model ID**
  selects the profile (auto-detection).
- **Unknown module → graceful degraded/generic mode**, not a crash, plus an
  invitation to contribute a profile.

No V31 (or any single-model) specifics may live in `core` logic, `apps/*`, or the
FFI. If it varies by module, it is a profile.

## Consequences
- Adding a module = authoring a profile (data) + parsing its Data List, ideally
  with **zero code changes**.
- Upfront cost: designing a robust, versioned profile schema and the parsers.
- The schema must version (`schema_version`, firmware ranges) to absorb address-map
  changes across modules/firmware.
- We can validate "profile ↔ address map" with contract tests, and ship the V31
  profile first while the architecture stays general.
- Reinforces [ADR-0001](0001-replace-not-mirror.md) (we model data, not screens).
