# Roadmap

Phased so that the **biggest unknowns are tested first** and each phase ships
something usable. Detail and rationale: [docs/SPEC.md](docs/SPEC.md); tactical
state: [TASKS.md](TASKS.md).

> One principle runs through every phase: **the app has no voice of its own.**
> It makes the *system* accessibility features work brilliantly — a complete
> accessibility tree for the user's screen reader (VoiceOver / TalkBack) plus
> announcements for what the screen reader can't see — and never reimplements
> them ([ADR-0014](docs/adr/0014-screen-reader-is-the-only-voice.md)).

## Phase 0 — Web MIDI PoC (throwaway, de-risk) — ✅ done
**Goal: prove the protocol and kill risk #1 before touching mobile.**
- TypeScript + Web MIDI in Chrome, over USB-C.
- Identity handshake; RQ1→DT1 round-trip; read `Current` / kit name / tempo;
  switch kit; edit a parameter.
- **Persistence answered on hardware:** a DT1 write to a kit address survives a
  power-cycle, no separate store action ([PROTOCOL §7](docs/PROTOCOL.md)).

## Phase 1 — Rust core + tests — ✅ done
- SysEx codec: RQ1/DT1, checksum, 4-byte 7-bit address arithmetic, nibble/signed/
  ASCII encodings, fragmented-SysEx reassembly. Golden vectors + proptest.
- Device profiles as data (`profiles/roland-v31.json`), Identity auto-detect,
  firmware policy; Fluent i18n in the core; the **write→readback→verify** session
  FSM; UniFFI export.
- Grew beyond the original scope: a **device-mock e2e harness** — a
  profile-driven `VirtualDevice` + virtual clock (timing races are first-class
  tests), NDJSON cassettes with golden-replay, and the simulated module exposed
  over FFI so the app runs the full pipeline with no hardware (`simffi`, B1).

## Phase 2 — First platform end-to-end (Apple) — ✅ MVP done
Built **iOS-first** (one multiplatform target: iPhone/iPad/Mac), not Android
first as originally planned.
- CoreMIDI transport (USB) ↔ core; robust endpoint selection.
- **MVP features:** connect + identify; current kit + tempo live (poll
  `Current`); kit switching; rename; accessible tempo editor (VoiceOver
  adjustable, edits shown in-progress until device-confirmed).
- Output per [ADR-0014](docs/adr/0014-screen-reader-is-the-only-voice.md): a
  clean accessibility tree; screen-reader **announcements** (routed by
  category/source — interrupting kit nav, no double-speech) for what the screen
  reader can't observe; earcons + haptics. No app TTS.
- **Validated live on the real V31**; UI tests drive the real pipeline against
  the simulated module (`--simulated-device`) + accessibility-audit gate.
- Ongoing gate, not a checkbox: the **eyes-closed screen-reader pass** for every
  feature (`just mac-run-sim` runs it hardware-free).

## Phase 3 — Data pipeline (catalogs)
- Parse the V31 Data List (local PDF) → our JSON: instrument catalog, FX/ambience
  types + presets, drum-kit list, parameter map (addresses/ranges/encodings,
  cross-checked vs the MIDI address map).
- Versioned/updatable (catalog grows with Roland Cloud expansions).
- Embed JSON in the core; expand the parameter map (pad/layer, FX, ambience).

## Phase 4 — Second platform (Android)
- MIDI transport (`android.media.midi`, USB-C) ↔ the same core.
- Accessible Compose UI at parity with the Apple MVP; TalkBack announcements via
  the platform channel (`announceForAccessibility` / live regions) + earcons +
  haptics — same rule, no app TTS ([ADR-0014](docs/adr/0014-screen-reader-is-the-only-voice.md)).
- Full TalkBack pass, eyes closed.

## Phase 5 — V1 editors
- Global settings (`Setup`), trigger/sensitivity (`Trigger`), full kit editor
  (rename, instrument per pad/layer, pitch/decay/transient, vol/pan, pad EQ/comp,
  sends), FX types + presets, ambience.
- Announce hardware edits live (Transmit Edit Data) — any parameter, not just
  kit/name/tempo.
- **Performance mode** (big no-aim targets) + optional push-to-talk voice
  commands (input, not output).
- Sessions with blind drummers before release.

## Cross-cutting (every phase)
- **Nonvisual-first** is the foundation, not a phase: each feature is built and
  tested **eyes-closed first**, with the visual layer added only afterward as a
  removable enhancement ([North Star](docs/SPEC.md#-north-star--nonvisual-first),
  [CONTRIBUTING.md](CONTRIBUTING.md)). "If it can't be done eyes-closed, it isn't
  done."
- **Feed the system, don't clone it:** expose state through the accessibility
  tree and the announcement channel; honour the user's screen-reader voice, rate,
  and verbosity — never run a second narrator (ADR-0014).
- No blind writes: what reaches the user is always the **read-back** value.
- Keep Roland PDFs out of the repo; ship only derived data.
