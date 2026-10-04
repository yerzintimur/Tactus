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

- [x] **`P1` CI (GitHub Actions).** Two jobs on every push/PR: the core gate
  (`just test-core` + `just test-e2e` in the same pinned Docker image as local
  dev) and the Apple leg (`just build-ios` → `just ios-gen` → simulator tests
  incl. the a11y audit) on a pinned `macos-26` runner. Same `just` recipes as
  local dev; README badge; docs/CI.md walkthrough. Add Android later (M5).
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
- [x] **`P2` Device-mock platform sim — B1 (Phase 4).** The core's
  `VirtualDevice` is exposed over FFI as `VirtualDeviceHandle` (cargo feature
  `simffi`, dev bindings only — `just build-ios-release` builds without it and
  asserts the symbols are absent). A DEBUG
  [SimulatedTransport.swift](apps/ios/TactusApp/SimulatedTransport.swift) plugs it
  into `CoreSession` through the new `MidiTransporting` seam; `--simulated-device`
  selects it at launch. The UI tests now drive the **real pipeline** (identify →
  poll → kit nav → tempo edit, all confirmed by read-backs) + the a11y audit, no
  hardware; `just mac-run-sim` launches the Mac app the same way for eyes-closed
  VoiceOver runs.
- [ ] **`P2` Device-mock platform sim — B2 (Phase 5).** A real virtual CoreMIDI
  endpoint the production `MidiTransport` connects to as if it were hardware
  (exercises packetization, enumeration, hot-plug, the 256-byte cap; Mac/device
  only, manual/nightly).

## M3 — Data pipeline (catalogs)

- [x] **`P2` PDF parsers** (`tools/parse_datalist.py`, `tools/parse_midi_impl.py`,
  `just data-derive`): drum kits (200), instruments (246 preset + 3 EXV packs =
  764), FX types (95) into `profiles/catalogs/roland-v31/`; the full MIDI
  Implementation §3 into `profiles/maps/roland-v31-address-map.json` (25 blocks,
  792 parameters). Fail-loud parsing + golden-fact validation; see
  [tools/README.md](tools/README.md).
- [x] **`P2` Populate V31 catalogs** and wire them in: embedded via
  `device::builtin_catalog_json` + typed `device::catalogs`;
  `model::InstrumentCatalog::v31()` speaks real names (35 = "DW Concrete S"),
  unknown numbers still degrade gracefully.
- [x] **`P2` Expand the V31 parameter map**: `dims` schema (per-layer/unit/pad/
  FX-slot repeats), `signed_nibble` encoding, and 21 new curated parameters
  (pad/layer instrument-bank-volume-pitch-decay, per-unit ambience sends, pan,
  bus FX type/switch, overhead/room/reverb) — every parameter carries a `doc`
  ref and is **cross-checked byte-for-byte against the parsed map** by
  `device/tests/map_crosscheck.rs`. Found & fixed: `sub_name` is 64 bytes, not
  16. *(Deferred: per-FX-type parameter names for the 95 MFX types. Expansion
  bank numbering read from the module 2026-10-04 — PROTOCOL §5.)*

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
- [x] **`P1` Kit navigation knows where the list ends.** `capabilities.kit_count`
  was inert data and `select_kit` never checked the parameter's range, so "next
  kit" at the module's last slot wrote a kit that doesn't exist: the module
  ignored it and the user got an edit **timeout** ("no response, check the
  connection") a second later; the only guard was `.disabled` in SwiftUI, i.e. in
  the wrong layer for Android to inherit. `Session::next_kit`/`previous_kit` now
  bound stepping with `DeviceProfile::max_kit_number` (the `current.kit_num` range,
  falling back to the declared count) and announce the edge — "Last kit." —
  as `KitNav`, writing nothing; a direct out-of-range `select_kit` is rejected
  locally instead of timing out. Pinned in
  [full_session.rs](core/crates/e2e/tests/full_session.rs).
- [x] **`P2` Set lists — read and rearrange.** A set list is the module's own
  ordering of kits (32 lists × 32 steps, `END` = −1) and the drummer's answer to
  "arrange the kits how I want"; on the V31 it is otherwise reachable only through
  the screen. Landed as data + core + screen: `setlist.name` / `setlist.step` in
  the profile (with the generic `ascii_nibble`, `sentinel` and `display_offset`
  mechanics they needed), `Session::read_setlist` and the step edits, and an
  accessible screen where each step reads "Step 1: 5 · Jazz" and carries move /
  remove as row actions. Reading is one RQ1 for the whole block; multi-write edits
  go one write at a time, each confirmed before the next
  ([PROTOCOL §5](docs/PROTOCOL.md), [setlists.rs](core/crates/e2e/tests/setlists.rs)).
  **Verified on the V31 (2026-10-04):** `END` is `0F 0F 0F 0F` on the wire, read
  and written; a 160-byte read comes back as one DT1; step and nibble-packed name
  writes round-trip (set list 32 was edited and restored byte-for-byte). The
  *active* set list is invisible over MIDI: stepping through one on the panel
  sends only Bank Select + Program Change, exactly like the kit knob, and no
  SysEx — so list position is the app's to drive, not to read (PROTOCOL §5).
- [x] **`P2` Step through a set list from the app.** The module cannot tell us
  which list or step it is on, so the position lives in the engine: "Next step" /
  "Previous step" select the step's kit through the ordinary verified kit-select
  path and, once the module confirms, announce "Step 2, Kit 5: Jazz" (`KitNav`,
  spoken even when the kit was already current — the drummer asked where they
  are). The list never wraps: the edges and an empty list are announced and write
  nothing. The position survives reopening the screen and follows the list when
  it is shortened; the row being played reads "…, current step"
  ([setlists.rs](core/crates/e2e/tests/setlists.rs), `SetlistState::target`).
- [x] **`P2` Notice the current kit's slot being replaced.** Copying or importing
  a kit over the slot you are standing on changes everything about it while its
  *number* stays put, so polling the number alone left the app naming the kit that
  used to be there — reporting stale state as fact, the one thing it must never
  do. The engine now re-reads the current kit's name every ~3 s, and on a change
  drops the rest of that slot's cached values and re-reads them; silent while the
  name is unchanged ([timed_scenarios.rs](core/crates/e2e/tests/timed_scenarios.rs)).
  Seen on hardware: the name read goes out every tenth poll (~3.3 s) and nothing
  is spoken while the kit is unchanged.
- [x] **`P1` One tick timer, not one per action.** Every user action asks the core
  for a tick (edits need ageing, selections a confirmation read) and the Mac app
  made a new timer for each request; a tick re-arms itself, so each action added a
  polling chain that never ended — on hardware the `Current` read ran nine times a
  second after three actions. `ScheduleTick` now means "re-arm the one timer"
  (documented on the effect, implemented by the Swift session and the e2e harness,
  pinned in [timed_scenarios.rs](core/crates/e2e/tests/timed_scenarios.rs) and
  the Swift unit tests).
- [x] **`P3` Program Change as a re-poll hint.** The module sends Bank Select +
  Program Change on channel 10 for every kit change it makes itself (panel knob,
  set-list step — seen 2026-10-04) and no SysEx. The core now reads `Current` the
  moment a Program Change arrives (not over an in-flight edit), so a panel kit
  change is announced within one read's latency instead of up to 300 ms later;
  the number in the message is never trusted as the kit — the poll stays the
  source of truth (PROTOCOL §6). The virtual device sends the same triple and no
  DT1 for its own kit changes, so the simulator and the UI tests exercise the
  real path ([timed_scenarios.rs](core/crates/e2e/tests/timed_scenarios.rs)).
  **On hardware** the read leaves in the same millisecond as the Program Change,
  but the module answers ~416 ms later together with the regular poll's reply:
  it is silent for ~400 ms after a kit change, so the gain over the 300 ms poll
  is marginal on this firmware (PROTOCOL §6). Kept as a cheap, harmless guard.
- [ ] **`P3` Set-list names without opening each list** — the picker offers
  "Set list 1…32" because the names live in the module and reading all 32 would be
  32 requests. Worth a background sweep (paced) once the hardware answers above.
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
  2. [x] **Platform** —
     [AnnouncementService.swift](apps/ios/TactusApp/AnnouncementService.swift)
     routes by the tags: `KitNav` → interrupting (high); `UserInitiated
     ParamEdit` → suppressed (VoiceOver voices the focused control → no
     double-speech; the earcon still plays); everything else keeps the core's
     priority. Routing decisions are static + unit-tested; `just ios-test` green
     (incl. the a11y audit). *(Eyes-closed VoiceOver pass still pending.)*
  3. [x] **UI** — the tempo adjustable
     ([ContentView.swift](apps/ios/TactusApp/ContentView.swift)) presents the edit
     as *in-progress* until the device confirms (`tempoEditInFlight` in
     [CoreSession.swift](apps/ios/TactusApp/CoreSession.swift)): the accessibility
     value reads "Updating…" instead of a stale number, the visible value dims
     with a spinner, and the confirmed (device-verified) value replaces it; on
     mismatch the core's `Error` announcement carries the actual value — no
     double-speech, no blind write.
  Remaining: **eyes-closed VoiceOver validation** (the authentic path) — needs the
  module (or the Phase-4 `SimulatedTransport`) so read-backs actually flow.
  AX/assistive-access for driving the app via the a11y tree is set up (Claude.app
  granted Accessibility).
- [ ] **`P3` Firmware `version_format`-aware display.** `FirmwareVersion::display`
  shows raw dotted `0.2.1.0`; the V31 renders `00 02 01 00` as **"0.2.10"** (last
  two bytes = one component). Make the display honour the profile's
  `version_format`. (Build suffix "(0031)" isn't in the Identity Reply.)
- [x] **`P2` Full Transmit Edit Data handling.** With the setting on, the module
  sends a DT1 to the parameter's own address for every panel edit (one per knob
  step; a snare edit sends head and rim as two). `DeviceProfile::locate` turns the
  address back into a parameter and its indices, and the core announces any such
  write as "Kit volume: 0.5 dB" — label and value, device-initiated, each step of
  a sweep interrupting the last on the platform; addresses the profile does not
  describe are ignored. Captured on the V31 2026-10-04 (PROTOCOL §6;
  [hardware_edits.rs](core/crates/e2e/tests/hardware_edits.rs),
  [locate.rs](core/crates/device/tests/locate.rs)) and **verified end to end in
  the Mac app** the same day: the kit-volume knob read "Kit volume: 0.5 dB … 1.5
  dB … 0.0 dB", a pad's volume "Pad volume: …" twice per step (head and rim).
  *Open:* pads have no names
  in the profile yet, so a pad edit says "Layer volume" without which pad; the
  value cache is per parameter id, so only grid-free parameters of the current
  kit land in the snapshot.
- [ ] **`P3` Transmit Edit Data is OFF out of the box** and not in the address
  map, so the app cannot switch it on. Onboarding and the connect summary should
  tell the user to enable it (SYSTEM → MIDI → BASIC); without it the app only
  learns of kit changes.
- [x] **`P3` Name the pads.** The profile's `dimensions` name every position a
  parameter repeats over (`unit` 1–28, `pad` 1–14, `layer` A–C, `fx` 1–8 by bus;
  set-list steps by number), from the MIDI Implementation's index tables; a
  panel edit is spoken from the pad outwards — "Snare rim, Layer A, Layer
  volume: 0.5 dB", cymbal zones as bow/edge. Not yet used by any screen: the
  pad/layer screens, when they come, take their row labels from the same data.
- [ ] **`P2` Low-vision pass** — high-contrast theme + Dynamic Type hardening;
  re-enable `contrast` + `dynamicType` in the audit gate
  ([apps/ios/README.md](apps/ios/README.md) explains why they're excluded now).

## M5 — Android (second platform)

- [ ] **`P1` Android scaffold** — Gradle/Compose/Kotlin project + cargo-ndk build
  of `libtactus.so` (arm64-v8a, x86_64) + JNA bindings.
- [ ] **`P1` Android MIDI transport** (`android.media.midi`, USB-C) ↔ core.
- [ ] **`P2` Android announcements** (`announceForAccessibility` / live regions,
  routed by the core's category + source per ADR-0014) + earcons + haptics. No
  app TTS.
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
- [x] **Per-segment mixed-language speech** — the core marks module-sourced text
  and reports `spans`; iOS posts announcements as an `NSAttributedString` with
  priority plus a language per range, so a Russian sentence quoting "Jazz Funk"
  is pronounced correctly ([ADR-0011](docs/adr/0011-mixed-language-speech.md)).
- [x] **The app's own interface is localized** — labels come from the same Fluent
  catalogs as the speech via a typed `UiString` enum over the FFI; a Rust test
  walks every variant in every locale, so a missing translation fails the build
  instead of being read aloud as an identifier.
- [x] **Language override** — device language by default, overridable in-app and
  persisted; the picker is built from the core's `available_locales`, each
  language under its own name.
- [ ] **`P3` Tag static labels with their language too** — a control whose label
  is a kit name still reads in the interface language. SwiftUI has no per-element
  language attribute, so this needs a UIKit-backed representable (ADR-0011).
- [x] **i18n keys for every profile parameter** — all 28 now carry one, with
  labels and value phrasing in both catalogs; enum values speak the module's own
  word (tagged English) rather than a raw number. A test walks every parameter in
  every locale, so an unlabelled parameter fails the build.
- [x] **`P2` Speak instrument and effect *names*, not numbers.** A parameter
  may bind a `catalog` (and, for the V31's instruments, the `catalog_bank`
  parameter that selects the bank at the same indices); `format_parameter` then
  speaks the catalogued name as device content — `kit.fx.type` 13 is "PHASER"
  end to end, read-back confirmation included. Instruments need both numbers:
  reading the module showed bank 0 = presets, bank 1 = SYNTH WAVE, bank 2006+n =
  pack EXVnnn, confirmed on all three documented packs and independent of the
  slot a pack is loaded in (PROTOCOL §5), so `format_parameter_in_bank(def, bank, number)`
  names "TR-808 Kick 1", and an uncatalogued pack says "Instrument #3 in bank
  2032 (unknown)" rather than guessing a preset. *Remaining for the kit editor:*
  read `inst_bank` alongside the instrument (same indices) and announce through
  the banked formatter — the engine's value cache is still keyed by parameter id
  alone, with no indices.
- [x] **`P2` The `-INF` sentinel reads as silence.** Raw −601 on every dB
  parameter is `-INF` on the module's screen — silence, not "−60.1 dB". The nine
  dB parameters now carry a `sentinel` (the mechanism the set-list `END` uses) and
  speak "Silent" / «Тишина»; the view-model's display range stops at the lowest
  number (−60.0 dB) while the raw range still includes the sentinel, so an
  adjustable can still reach it. A cross-check test demands a sentinel wherever
  the doc's display column names a value before the numbers, so a future level
  can't be added without one
  ([map_crosscheck.rs](core/crates/device/tests/map_crosscheck.rs)). **Verified
  on the V31 (2026-10-04):** −601 is accepted and reads back as `0F 0D 0A 07`
  (PROTOCOL §4).
- [x] **Second module studied — Roland TD-17** ([notes](docs/devices/roland-td-17.md)).
  A blind drum teacher we can reach uses a TD-17KVX2, which makes it the first
  real second target. Same Roland SysEx mechanics (Model ID `00 00 00 4B`,
  4 bytes vs the V31's 3 — the codec already takes a slice); its address map is
  derived and committed (`profiles/maps/roland-td-17-address-map.json`, 4 areas /
  17 blocks / 163 params + 482 in variant tables) by the *same* parser, which
  needed five generic robustness fixes and no module-specific code.
- [ ] **`P3` TD-17 device profile** — `profiles/roland-td-17.json` + a cross-check
  test; data only, except one `version_format` variant: the TD-17's Identity Reply
  carries a *coded* software revision (`00 00 00 02` = v2.00), not raw digits.
- [ ] **`P3` TD-17 catalogs** — kit/instrument lists from its Data List; needs real
  work in `parse_datalist.py`, which is tuned to the V31's page geometry.
- ~~BLE-MIDI transport~~ — **deferred, not planned**
  ([ADR-0015](docs/adr/0015-usb-midi-only.md)). USB is the only supported
  transport on both platforms and both modules. Consequence to keep in view: the
  TD-17 has no MIDI IN jack, so USB is its only path — a USB-C→USB-B cable from
  Android or a USB-C iPhone, an adapter only on Lightning.

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
