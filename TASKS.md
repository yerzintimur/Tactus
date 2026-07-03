# Tactus — Task backlog

Living, tactical checklist. Companion to [ROADMAP.md](ROADMAP.md) (strategic
phases + rationale), [docs/SPEC.md](docs/SPEC.md) (what/why), and
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) (how). Tick `[x]` when a task is done
and verified; keep this file honest about real state.

**Priorities:** `P0` urgent · `P1` high · `P2` medium · `P3` low.
**Convention:** every feature is *done* only when usable eyes-closed
([North Star](docs/SPEC.md#-north-star--nonvisual-first)); no blind writes.

---

## Done (built & tested)

- [x] **Rust core — `sysex`**: RQ1/DT1/Identity framing, Roland checksum, 4-byte
  7-bit address arithmetic, encodings (plain7/nibble/signed/ASCII), fragmented
  SysEx reassembly. Golden vectors + proptest.
- [x] **Rust core — `device`**: `DeviceProfile` schema, `ProfileRegistry`,
  Identity-Reply auto-detect, firmware policy (detect/announce/never-block).
- [x] **Rust core — `model`**: parameter formatting, Fluent i18n (`en`/`ru`),
  instrument-catalog type (data still empty), intents.
- [x] **Rust core — `engine`**: connect→identify→ready FSM, `Current` polling,
  **write→readback→verify** edit pipeline, event/effect emission. Full test
  suite against a fake module.
- [x] **Rust core — `ffi`**: UniFFI surface (`TactusSession`, effects, events).
- [x] **V31 profile as data** ([profiles/roland-v31.json](profiles/roland-v31.json))
  — MVP subset: 5 parameters; `catalogs` + `firmware.tested` still empty.
- [x] **Apple app (iPhone/iPad/Mac) MVP**: connect, identify, kit nav
  (prev/next), rename, speech (best installed voice), earcons/haptics,
  accessibility-audit gate.
- [x] **Protocol validated live on the real V31** (over the Mac as USB host).
- [x] Build pipeline: Docker for core, cargo-swift XCFramework, XcodeGen, just.

---

## M1 — Validate on the real V31 (do now, hardware is live)

- [x] **`P0` Resolve persistence (risk #1).** **Resolved live (2026-06-15):** a
  DT1 write to a kit address **persists across a power-cycle with no separate
  save** — set kit 16 tempo 120.1, power-cycled, read back 120.1
  ([PROTOCOL §7](docs/PROTOCOL.md)). No `SNAPSHOT SAVE` needed for kit-common
  edits. *(Spot-check a second parameter family before relying universally.)*
- [x] **`P1` Validate the edit pipeline live.** Tempo edit verified end-to-end on
  hardware (write→read-back→actual value); **tempo offset confirmed `0x6C`** (not
  `0x6D`/`0x6F` — profile was right). Hot-plug connect-first/power-later +
  destination selection also validated live. *(Surfaced two real bugs — see M4.)*
- [x] **`P2` Capture V31 firmware version + byte format.** Live Identity Reply
  `…03 00 00 02 01 00 F7` → bytes **`00 02 01 00`** = module "0.2.10 (0031)".
  `firmware.tested` now lists `[0,2,1,0]`. Build "(0031)" isn't in the Identity
  Reply. *(Follow-up: `version_format`-aware display — see M4.)*
- [x] **`P2` Robust MIDI endpoint selection.** Replaced the `destination[0]`
  heuristic with a scored policy in
  [MidiTransport.swift](apps/ios/TactusApp/MidiTransport.swift): prefer a
  bidirectional port on a device we also receive from (the module), then real
  hardware over software buses (IAC / Network Session), skipping offline
  endpoints. Policy extracted to a pure, unit-tested `selectDestination`;
  groundwork for multi-device (M7). *(Confirm on hardware with the real V31.)*

## M2 — Engineering hygiene

- [ ] **`P1` CI (GitHub Actions).** Core `fmt`/`clippy`/`test` in Docker + iOS
  build & tests (incl. the a11y audit) on a macOS runner. Add Android later.
- [ ] **`P3` Make engine timings tunable** (poll 300 ms, identity-retry 900 ms,
  edit-timeout) — currently hardcoded in `engine/src/session.rs`.
- [x] **`P1` Device-mock e2e foundation (Phases 1–2).** A profile-driven
  `VirtualDevice` (a dumb, address-keyed byte store that answers Identity/RQ1/DT1,
  persists writes, and emits unsolicited hardware pushes — works for any profile by
  data) + a virtual-clock `Harness` that interleaves delayed device replies and
  scheduled ticks on one deterministic timeline, so **timing** bugs are first-class:
  bug B ([PROTOCOL §6](docs/PROTOCOL.md)) is reproduced as a passing test the old
  synchronous `drive()` could not express. Plus a **cassette** format (NDJSON under
  [tools/cassettes/](tools/cassettes/)) with golden-replay. New crates
  [devicesim](core/crates/devicesim/) + [e2e](core/crates/e2e/) (`just test-e2e`);
  the engine's `FakeModule`/`drive()` is removed and its tests ported.
- [ ] **`P2` Device-mock recorder (Phase 3, needs hardware).** Authoritative in-app
  `RecordingTransport` tap (true CoreMIDI timestamps + fragmentation + action
  annotations) writing cassettes, plus a `tools/` `log stream` parser (lossy
  fallback). Record real V31 sessions into [tools/cassettes/](tools/cassettes/); the
  Phase-2 golden replay then validates `VirtualDevice` against hardware (fix until
  byte-for-byte) and calibrates `TimingProfile` from the captures.
- [ ] **`P2` Device-mock platform sims (Phases 4–5).** **B1:** a Swift
  `SimulatedTransport` backed by `VirtualDevice` over a debug-only `simffi` FFI
  object, driving eyes-closed XCUITest/VoiceOver in CI. **B2:** a real virtual
  CoreMIDI endpoint the production `MidiTransport` connects to as if it were hardware
  (exercises packetization, enumeration, hot-plug, the 256-byte cap; Mac/device
  only, manual/nightly).

## M3 — Data pipeline (catalogs)

- [ ] **`P2` Data List PDF parser** (`tools/`, currently empty): emit
  instruments / FX / ambience JSON; cross-check the parameter address map.
- [ ] **`P2` Populate V31 catalogs** (`profiles/catalogs/roland-v31/*.json`) and
  wire them into the profile + `model`.
- [ ] **`P2` Expand the V31 parameter map**: pad/layer, FX, ambience parameters
  (from the MIDI Implementation).

## M4 — Finish the Apple app toward V1

- [x] **`P1` Expose current value + range/scale in FFI** (snapshot / view-model).
  `engine::Session::snapshot()` → `Snapshot { connection, device, current_kit,
  parameters }`; each `ParameterView` carries the last device-confirmed value,
  localized label + display, and numeric range/scale/step (raw + display units).
  Read-through value cache in the engine (device is source of truth). Mirrored
  over UniFFI + tested. **Next:** regenerate Swift bindings (`just gen-bindings`)
  to consume it in the app.
- [x] **`P2` Tempo editor UI** — accessible adjustable (VoiceOver swipe ↑↓, 0.1
  BPM steps) + visible −/+ buttons, routed through write→readback→verify. Value
  shown is the device-confirmed one (no blind writes); spoken confirmation comes
  from the core. Projected from `snapshot()` in
  [CoreSession.swift](apps/ios/TactusApp/CoreSession.swift); unit + a11y-audit
  gated. *(Live value round-trip — incl. tempo offset `0x6F` — validated under
  M1 P1 on hardware.)*
- [x] **`P1` BUG (live): `select_kit` "value unknown" — fixed.** Kit selection no
  longer verifies through the address-keyed edit pipeline (its verify slot at
  `00 00 00 00` collided with the poller — [PROTOCOL §6](docs/PROTOCOL.md)).
  `select_kit` writes the DT1 and confirms via the regular `Current` read path:
  stale (unchanged) reads are ignored while the selection is in flight, the
  actual landed kit is announced, and a tick-driven timeout keeps a failed select
  audible. Race + timeout + rejected-write scenarios pinned deterministically in
  [timed_scenarios.rs](core/crates/e2e/tests/timed_scenarios.rs). *(Verify on
  hardware at the next session.)*
- [ ] **`P1` Speech model → "the screen reader is the only voice"**
  ([ADR-0014](docs/adr/0014-screen-reader-is-the-only-voice.md)). Live testing
  reframed two bugs (speech flood on hardware kit-scroll; double-speech on a UI
  tempo edit) into one principle: the user's screen reader is the single voice;
  the app exposes the a11y tree and announces only screen-reader-invisible changes,
  **interrupting** (not debouncing) for kit nav, with no double-speech. Stages:
  1. [x] **Core:** `Speech` carries `category`
     (`Connection`/`KitNav`/`ParamEdit`/`Error`/`Info`) + `source`
     (`DeviceInitiated`/`UserInitiated`); every emission is tagged (the initial
     kit after connect is `Connection` — part of the summary, not a `KitNav`
     barge-in); mirrored over FFI; tags pinned in
     [full_session.rs](core/crates/e2e/tests/full_session.rs). The stale
     read-back gate in [session.rs](core/crates/engine/src/session.rs) **stays**
     — per the ADR's resolved edge case it is device-as-truth *content* gating
     (a kit we scrolled past must not be cached or announced as current);
     interruption is the platform's announcement policy, not an engine drop.
  2. [ ] **Platform** —
     [AnnouncementService.swift](apps/ios/TactusApp/AnnouncementService.swift)
     (already a pure announcement router; the app has no TTS of its own) routes by
     the tags: interrupt for `KitNav`; **suppress** `UserInitiated ParamEdit`
     (VoiceOver voices the focused control → no double-speech). Needs regenerated
     bindings (`just build-ios`).
  3. [ ] **UI** — the tempo adjustable
     ([ContentView.swift](apps/ios/TactusApp/ContentView.swift)) shows the edit as
     *in-progress* until the device confirms; the screen reader voices the verified
     value (or a `DeviceInitiated` correction on mismatch) — no double-speech, no
     blind write.
  Test eyes-closed via **VoiceOver** (the authentic path). AX/assistive-access for
  driving the app via the a11y tree is set up (Claude.app granted Accessibility).
- [ ] **`P3` Firmware `version_format`-aware display.** `FirmwareVersion::display`
  shows raw dotted `0.2.1.0`; the V31 renders `00 02 01 00` as **"0.2.10"** (last
  two bytes = one component). Make the display honour the profile's
  `version_format`. (Build suffix "(0031)" isn't in the Identity Reply.)
- [ ] **`P2` Full Transmit Edit Data handling** — announce any
  hardware-initiated edit, not just kit/name/tempo.
- [ ] **`P2` Low-vision pass** — high-contrast theme + Dynamic Type hardening;
  re-enable `contrast` + `dynamicType` in the audit gate
  ([apps/ios/README.md](apps/ios/README.md) explains why they're excluded now).

## M5 — Android (second platform)

- [ ] **`P1` Android scaffold** — Gradle/Compose/Kotlin project + cargo-ndk build
  of `libtactus.so` (arm64-v8a, x86_64) + JNA bindings.
- [ ] **`P1` Android MIDI transport** (`android.media.midi`, USB-C) ↔ core.
- [ ] **`P2` Android TTS** (TextToSpeech) + earcons + haptics; screen-reader-aware
  routing.
- [ ] **`P2` Android accessible UI** (Compose): connect, kit nav, rename — parity
  with the Apple MVP.
- [ ] **`P2` Android a11y gate** — ATF in Compose tests + manual TalkBack pass.

## M6 — V1 editors (full)

- [ ] **`P2` Global settings editor** (`Setup`: outputs, click/metronome, misc).
- [ ] **`P2` Trigger / sensitivity editor** (`Trigger`, 16 banks).
- [ ] **`P2` Full kit editor**: instrument per pad/layer, pitch/decay/transient,
  volume/pan, pad EQ/comp, sends.
- [ ] **`P2` FX editor** — choose effect *type* + presets (not raw numeric params).
- [ ] **`P2` Ambience editor** — room / overhead / reverb / resonance.
- [ ] **`P2` Performance mode** — a few huge no-aim targets (next/prev kit).
- [ ] **`P3` Push-to-talk voice commands** (on-device, small grammar) — optional.

## M7 — Multi-device & i18n depth

- [ ] **`P3` Multiple device instances** — endpoint `uniqueID` + user label,
  multiple concurrent sessions ([ADR-0010](docs/adr/0010-device-instances-and-source-of-truth.md)).
- [ ] **`P3` Per-segment mixed-language speech** — `LocalizedText` spans
  ([ADR-0011](docs/adr/0011-mixed-language-speech.md)).
- [ ] **`P3` Second device profile** (V51/V71, when HW/docs available) — prove the
  extension is data-only, no code changes.
- [ ] **`P3` BLE-MIDI transport** (secondary).

## M8 — Vision (deferred)

- [ ] **`P3` Data-driven UI renderer** — declarative view-model + generic native
  renderer ([ADR-0013](docs/adr/0013-data-driven-ui-renderer.md)).
- [ ] **`P3` Profile-pack backend + on-demand download**
  ([ADR-0012](docs/adr/0012-scope-and-generalization-path.md)).

## Cross-cutting

- [ ] **`P2` Hardware-in-the-loop test harness** — round-trip, persistence,
  live-edit against a real module.
- [ ] **`P1` Sessions with blind drummers** — release gate; automated checks can
  pass while the app is unusable.
- [ ] **`P3` Cosmetic renames** — repo `v31-vision` → `tactus`, `apps/ios` →
  `apps/apple`.
