# Tactus

[![CI](https://github.com/yerzintimur/tactus/actions/workflows/ci.yml/badge.svg)](https://github.com/yerzintimur/tactus/actions/workflows/ci.yml)

**An accessibility-first companion app for blind and low-vision drummers**,
starting with the Roland V31 drum module. Choose kits, read and edit settings, and
build custom kits — all by ear, through your phone's screen reader, speech, and
voice, without ever needing to see the module's screen.

The Roland V31 (TD-313 / TD-316 / VAD316 kits) has a colour screen and **no
built-in accessibility**. Tactus makes the phone the accessible interface and the
module a headless sound engine, talking over MIDI SysEx.

## What "Tactus" means

Tactus takes its name from two meeting ideas. In Renaissance music, the tactus was
the steady beat a conductor marked with the hand — the felt pulse that held an
ensemble together before anyone wrote it on a page. In Latin, the same word simply
means touch: perception through the body rather than the eye. That intersection is
the whole point. Tactus is built blind-first, for drummers who play by ear and feel
rather than by reading a screen. It turns your drum module into something you can
hear and control by voice, so the instrument speaks back to you — its kits, its
settings, its sound — and gets out of the way so you can keep time. The name isn't
about what's missing. It's about the senses that were always doing the real work.

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
| **[docs/HARDWARE_TESTING.md](docs/HARDWARE_TESTING.md)** | Is testing against a real V31 safe? (yes) + the per-session safety/debug protocol. |
| **[docs/CI.md](docs/CI.md)** | How CI works (GitHub Actions from scratch) + a primer on future mobile releases. |
| **[ROADMAP.md](ROADMAP.md)** | Phased plan: PoC → MVP → V1. |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | The Nonvisual-first contributor gate + PR checklist. |
| **[docs/adr/](docs/adr/)** | Architecture decision records (incl. [ADR-0006: Nonvisual-first](docs/adr/0006-nonvisual-first.md)). |

---

## Architecture at a glance

- **Rust core** (platform-agnostic, no I/O): SysEx codec, checksum, address math,
  typed parameter model, value⇄string for what the screen reader reads, and the
  **write→readback→verify** state machine. Exposed to mobile via **UniFFI**.
- **Thin native layers:** iOS (Swift/SwiftUI, CoreMIDI) and Android
  (Kotlin/Compose, `android.media.midi`) — accessible UI, MIDI transport, and
  screen-reader announcements. The app has **no voice of its own**: it feeds the
  system's accessibility features (VoiceOver / TalkBack), never reimplements them
  ([ADR-0014](docs/adr/0014-screen-reader-is-the-only-voice.md)).
- **Data:** instrument/FX catalogs + parameter map parsed from the V31 Data List
  into our own JSON, embedded in the core.

See [SPEC §6](docs/SPEC.md#6-system-architecture).

---

## Connecting the module

The V31 is class-compliant USB MIDI; the port on the module is **USB-C**.

- **iPhone / iPad with USB-C** — a regular USB-C ↔ USB-C data cable.
- **iPhone / iPad with Lightning** — Apple's
  [Lightning to USB 3 Camera Adapter](https://www.apple.com/shop/product/mx5j3am/a/lightning-to-usb-3-camera-adapter)
  **plus a USB-A → USB-C cable**: the adapter's port is USB-A, the module end is
  USB-C. Verified working with a real V31.
- **Mac** — a direct USB-C cable, no adapter.

---

## Roland documentation (not in this repo)

This project is built against two Roland documents — the **V31 MIDI
Implementation (v2.00)** and the **V31 Data List** — which are © Roland and are
**not redistributed here**. Download them yourself and drop them in
`docs/vendor/`; see **[docs/vendor/README.md](docs/vendor/README.md)** for links
and the reasoning. The repo ships only our own derived data and notes.

---

## Free, forever

Tactus is free, and it will stay free. Sighted musicians have never had to pay a
toll to read the screen on their own gear — that access is simply built in. Blind
and visually impaired drummers deserve the same: not a charity, not a premium tier,
not a subscription standing between a person and the instrument they already own.
Accessibility shouldn't be a paid upgrade. Tactus exists to close that gap, so that
a blind drummer can browse kits, change settings, and build their own sound with
exactly the independence everyone else takes for granted. No paywalls, no ads, no
locked features — just equal access to the instrument, full stop.

Running the project does carry real costs, though — developer accounts, hosting for
the instrument catalog, and the ongoing work of supporting more modules. That's
where supporters come in. If you believe blind and visually impaired musicians
deserve the same access to their instruments as everyone else, and you'd like to
help keep Tactus alive and growing, donations are welcome and deeply appreciated.
This is aimed at people who want to back the cause — not at the musicians who rely
on the app. If Tactus is your tool for playing, you owe nothing, ever. The app is
yours in full, exactly as it is for everyone else.

> _Donation links: TBD — to be added before public release._

## Status

Early development, **iOS-first**, targeting the **Roland V31**. The shared Rust
core (SysEx codec, device profiles, sans-I/O session engine, i18n) is implemented
and tested — including a **simulated module** for hardware-free end-to-end runs —
and the **Apple-first MVP** (connect, kit navigation, tempo editing, all through
the screen reader) has been **validated against a real V31**, on the Mac and on an
iPhone. See [ROADMAP.md](ROADMAP.md).

## License

[Apache-2.0](LICENSE). See `NOTICE` for trademark/affiliation disclaimers.

## Contributing

The bar is **Nonvisual-first**: every change must be fully usable **eyes-closed**
(verified with VoiceOver *and* TalkBack) before the visual layer is even
considered. Read **[CONTRIBUTING.md](CONTRIBUTING.md)** for the required workflow
and PR checklist.
