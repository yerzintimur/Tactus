# Roadmap

Phased so that the **biggest unknowns are tested first** and each phase ships
something usable. Detail and rationale: [docs/SPEC.md](docs/SPEC.md).

## Phase 0 — Web MIDI PoC (throwaway, de-risk)
**Goal: prove the protocol and kill risk #1 before touching mobile.**
- TypeScript + Web MIDI in Chrome, over USB-C.
- Identity handshake; RQ1→DT1 round-trip.
- Read `Current` → kit number; read kit name + tempo from `KitCommon`.
- Switch kit; edit one parameter.
- **Answer persistence:** does a DT1 write to a kit address survive a power-cycle,
  or is a separate store action required? (SPEC §14, risk #1.)
- Output: a short findings note that updates SPEC/PROTOCOL.

## Phase 1 — Rust core + tests
- SysEx codec: build/parse RQ1 & DT1; checksum; 4-byte 7-bit address arithmetic;
  nibble pack/unpack; signed & ASCII encodings.
- **Golden-vector unit tests** (PROTOCOL §3) + value⇄string round-trips +
  write→readback→verify state machine against a simulated module.
- Codec fuzzing (never panic on malformed SysEx).
- UniFFI export; smoke-test bindings.

## Phase 2 — Data List parser
- Parse the V31 Data List (local PDF) → our JSON: instrument catalog, FX/ambience
  types + presets, drum-kit list, parameter map (addresses/ranges/encodings,
  cross-checked vs the MIDI address map).
- Versioned/updatable (catalog grows with Roland Cloud expansions).
- Embed JSON in the core.

## Phase 3 — One platform end-to-end (Android first)
- MIDI transport (`android.media.midi`, USB-C) ↔ core.
- TTS (`TextToSpeech`) with the managed queue + earcons + haptics (SPEC §11).
- **MVP features:** connect; speak current kit + tempo (live, poll Current);
  switch kit from an accessible list.
- Full TalkBack pass, eyes closed.

## Phase 4 — Second platform
- iOS (CoreMIDI + AVSpeechSynthesizer) over the same core.
- Full VoiceOver pass.

## Phase 5 — V1 editors
- Global settings (`Setup`), trigger/sensitivity (`Trigger`), full kit editor
  (rename, instrument per pad/layer, pitch/decay/transient, vol/pan, pad EQ/comp,
  sends), FX types + presets, ambience.
- Live announce of hardware edits (Transmit Edit Data).
- **Performance mode** (big no-aim targets) + optional push-to-talk voice
  commands.
- Sessions with blind drummers before release.

## Cross-cutting (every phase)
- **Nonvisual-first** is the foundation, not a phase: each feature is built and
  tested **eyes-closed first**, with the visual layer added only afterward as a
  removable enhancement ([North Star](docs/SPEC.md#-north-star--nonvisual-first),
  [CONTRIBUTING.md](CONTRIBUTING.md)). "If it can't be done eyes-closed, it isn't
  done."
- No blind writes: speak the read-back value.
- Keep Roland PDFs out of the repo; ship only derived data.
