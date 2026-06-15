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

- [ ] **`P0` Resolve persistence (risk #1).** Does a DT1 write to a kit address
  survive a power-cycle, or is a separate store / `SNAPSHOT SAVE` required?
  Record findings in [docs/PROTOCOL.md](docs/PROTOCOL.md); until resolved the UI
  must clearly distinguish *"changed (live)"* from *"saved to slot"*
  (SPEC §14).
- [ ] **`P1` Validate the edit pipeline live.** Exercise select-kit and
  rename-kit end-to-end on hardware: write → read-back → spoken value must be the
  **actual stored** value. Confirm tempo offset `0x6F` (not `0x6D`).
- [ ] **`P2` Capture V31 firmware version + byte format.** Read the live Identity
  Reply; nail `version_format` and populate `firmware.tested` in the profile.
- [ ] **`P2` Robust MIDI endpoint selection.** Replace the `destination[0]`
  heuristic in [MidiTransport.swift](apps/ios/TactusApp/MidiTransport.swift:116);
  groundwork for multi-device (M7).

## M2 — Engineering hygiene

- [ ] **`P1` CI (GitHub Actions).** Core `fmt`/`clippy`/`test` in Docker + iOS
  build & tests (incl. the a11y audit) on a macOS runner. Add Android later.
- [ ] **`P3` Make engine timings tunable** (poll 300 ms, identity-retry 900 ms,
  edit-timeout) — currently hardcoded in `engine/src/session.rs`.

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
- [ ] **`P2` Tempo editor UI** — accessible adjustable (VoiceOver swipe ↑↓, 0.1
  BPM steps) routed through write→readback→verify. *(FFI snapshot ready: tempo's
  `numeric.range` gives min/max + 0.1 BPM `display_step`)*
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
