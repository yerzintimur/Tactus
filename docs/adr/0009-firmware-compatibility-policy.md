# ADR-0009: Firmware compatibility — detect, announce, never block

**Status:** Accepted · **Date:** 2026-06-07

## Context
Our address maps and behaviour are validated against **specific firmware versions**
of a module. Roland ships firmware updates; a newer firmware than we've tested could
(in principle) move an address or change behaviour. We need to (a) know the
connected firmware, (b) tell the user when it's outside what we've tested, and
(c) — per the maintainer's decision — **still let them use the app**.

Good news: we get the firmware **over our existing transport**. The Universal
**Identity Reply** (`F0 7E dd 06 02 … vv vv vv vv F7`), which we already request
during the connect/identify handshake, carries a **4-byte software-version field**.
The `sysex` codec already returns it as `IdentityReply.version`. No extra round-trip.

## Decision
1. **Capture firmware** from the Identity Reply: `device::FirmwareVersion([u8; 4])`,
   comparable (tuple order) with a best-effort `display()`. The exact mapping of the
   4 bytes to a human string (`version_format`) is **verified on hardware** per
   module.
2. **Each `DeviceProfile` declares the firmware it was tested against**
   (`firmware.tested`). The registry computes
   `profile.firmware_support(version) -> FirmwareSupport`:
   - `Tested` — version is in the tested set;
   - `UntestedNewer` — newer than the newest tested (e.g. a new firmware shipped);
   - `UntestedOlder` — older than the oldest tested;
   - `Unknown` — unparseable / no tested versions recorded yet.
3. **Never block.** For any non-`Tested` result the app **works exactly as normal**.
   At connect it **announces** the situation (a `High`-priority spoken message) and
   keeps it available in a "Device info" view for re-query. Example:
   *"Connected to V31, firmware 1.10. This firmware is newer than Tactus has been
   tested with — it should work; please report anything off."*
4. The result is surfaced in the FFI as `DeviceInfo.firmware` +
   `DeviceInfo.firmware_support` and on the `DeviceIdentified` event.

## Rationale
- A blind user must never be silently locked out of their own instrument because a
  firmware number didn't match a list — that would reproduce exactly the
  exclusion Tactus exists to remove. Inform, don't gate.
- Most firmware bumps don't touch the parameter map; defaulting to "works, with a
  heads-up" is the right risk trade-off, and the announcement sets expectations and
  drives bug reports that let us promote a version to `tested`.

## Consequences
- `device` implements `FirmwareVersion` + `FirmwareSupport` +
  `profile.firmware_support()` (task #5); the V31 profile records its tested
  firmware once read on hardware (tasks #6, #14).
- `engine` emits the compatibility announcement at connect (task #8).
- Verify on HW: the 4-byte version format and the actual V31 firmware value.
- Builds on [ADR-0007](0007-device-profile-abstraction.md) (profiles as data) and
  the never-block spirit of unknown-device handling.
