# V31 Vision

> **Working name** — not final (see [SPEC §14](docs/SPEC.md#14-risks-and-open-questions)).
> The starter doc called it "V31 Voice".

**An accessible companion app for blind and low-vision drummers using the Roland
V31 drum module.** Choose kits, read and edit settings, and build custom kits —
all by ear, through your phone's screen reader and speech, without ever needing to
see the module's screen.

The Roland V31 (TD-313 / TD-316 / VAD316 kits) has a colour screen and **no
built-in accessibility**. This project makes the phone the accessible interface
and the module a headless sound engine, talking over MIDI SysEx.

## ⭐ Philosophy: Nonvisual-first

> The nonvisual experience **is** the product. Everything — UI, features, tests —
> is built **first** to be fully usable **eyes-closed** (screen reader + speech +
> earcons + haptics). The **visual UI is a removable enhancement** for sighted
> helpers and low-vision users; it is **never required** to do anything.

This is *progressive enhancement from the nonvisual baseline* — the deliberate
inverse of "build it visual, then bolt on accessibility." Litmus test:
**"if it can't be done eyes-closed, it isn't done."** See the
[North Star](docs/SPEC.md#-north-star--nonvisual-first),
[ADR-0006](docs/adr/0006-nonvisual-first.md), and the contributor gate in
[CONTRIBUTING.md](CONTRIBUTING.md).

> ⚠️ **Independent project — not affiliated with, endorsed by, or supported by
> Roland Corporation.** "Roland", "V-Drums", and "V31" are trademarks of Roland
> Corporation, used only to describe interoperability. See `NOTICE`.

---

## Documentation

| Doc | What's in it |
|-----|--------------|
| **[AGENTS.md](AGENTS.md)** | Orientation for AI agents & contributors: non-negotiables, language policy, multi-device architecture, stack/versions, layout. |
| **[docs/SPEC.md](docs/SPEC.md)** | Full project spec: architecture, domain model, feature scope, risks, milestones. **Start here.** |
| **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)** | Deep development/architecture spec: device profiles, core crates, UniFFI contract, i18n, build/CI, versions, trade-offs. *(Russian, temporary.)* |
| **[docs/ACCESSIBILITY.md](docs/ACCESSIBILITY.md)** | How we serve blind users; the input-modality decision; concrete a11y requirements & testing. **Read if you've never built for blind users.** |
| **[docs/PROTOCOL.md](docs/PROTOCOL.md)** | V31 MIDI/SysEx: framing, checksum, addresses, value encodings, **golden test vectors**. |
| **[ROADMAP.md](ROADMAP.md)** | Phased plan: PoC → MVP → V1. |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | The Nonvisual-first contributor gate + PR checklist. |
| **[docs/adr/](docs/adr/)** | Architecture decision records (incl. [ADR-0006: Nonvisual-first](docs/adr/0006-nonvisual-first.md)). |

---

## Architecture at a glance

- **Rust core** (platform-agnostic, no I/O): SysEx codec, checksum, address math,
  typed parameter model, value⇄string for TTS, and the **write→readback→verify**
  state machine. Exposed to mobile via **UniFFI**.
- **Thin native layers:** iOS (Swift/SwiftUI, CoreMIDI, AVSpeechSynthesizer) and
  Android (Kotlin/Compose, `android.media.midi`, `TextToSpeech`) — accessible UI,
  MIDI transport, speech.
- **Data:** instrument/FX catalogs + parameter map parsed from the V31 Data List
  into our own JSON, embedded in the core.

See [SPEC §6](docs/SPEC.md#6-system-architecture).

---

## Roland documentation (not in this repo)

This project is built against two Roland documents — the **V31 MIDI
Implementation (v2.00)** and the **V31 Data List** — which are © Roland and are
**not redistributed here**. Download them yourself and drop them in
`docs/vendor/`; see **[docs/vendor/README.md](docs/vendor/README.md)** for links
and the reasoning. The repo ships only our own derived data and notes.

---

## Status

Specification draft. No code yet. Next concrete step: a throwaway **Web MIDI PoC**
to prove the SysEx round-trip and answer the persistence question
([ROADMAP](ROADMAP.md)).

## License

[Apache-2.0](LICENSE). See `NOTICE` for trademark/affiliation disclaimers.

## Contributing

The bar is **Nonvisual-first**: every change must be fully usable **eyes-closed**
(verified with VoiceOver *and* TalkBack) before the visual layer is even
considered. Read **[CONTRIBUTING.md](CONTRIBUTING.md)** for the required workflow
and PR checklist.
