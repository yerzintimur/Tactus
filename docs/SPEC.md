# Tactus — Project Specification

> Project name: **Tactus** (the repo directory is still `v31-vision`; rename is
> cosmetic and deferred). Status: **Specification draft.** This document is the
> source of truth for *what* we build and *why*. Companion docs:
> [ACCESSIBILITY.md](ACCESSIBILITY.md) (how we serve blind users),
> [PROTOCOL.md](PROTOCOL.md) (V31 MIDI/SysEx, derived facts + golden vectors).

---

## ⭐ North Star — Nonvisual-first

**This is the foundation everything else rests on.** Read it before anything else.

> **Nonvisual-first:** the complete, primary experience of this app is
> **nonvisual** — fully operable with the screen reader, speech, earcons, and
> haptics, **eyes closed**. The visual UI exists only as a **secondary
> enhancement** for sighted helpers (teachers, parents, techs) and low-vision
> users. **Visual UI is never required to do anything.** Turn the screen off and
> the app still does its whole job.

This is **progressive enhancement starting from the nonvisual baseline** — the
deliberate inverse of the usual "build it visual, then bolt on accessibility"
(graceful degradation). We build the nonvisual path first; the visual layer is
added on top and must be removable without losing any function.

**The order of work for every feature, every time:**

1. **Design** the nonvisual interaction (what is spoken, what gestures, what
   earcons/haptics, what the read-back says).
2. **Build** it nonvisually.
3. **Test** it **eyes closed** with VoiceOver *and* TalkBack — including the
   automated assertions on labels/values/announcements (§16).
4. **Only then** add or refine the **visual** affordances for sighted/low-vision
   users — and re-verify the nonvisual path still stands alone.

**Definition of done (project-wide):** a feature is not "done" until it is fully
usable eyes-closed. A change that works only visually is **incomplete by
definition**, not a separate "accessibility task for later." Tests assert the
nonvisual experience first (§16); visual checks are secondary.

> **Litmus test:** *"If it can't be done eyes-closed, it isn't done."*

See [ADR-0006](adr/0006-nonvisual-first.md) and the contributor gate in
[CONTRIBUTING.md](../CONTRIBUTING.md).

---

## 1. Summary

An accessible companion app (iOS + Android) that lets a **blind or low-vision
drummer fully operate a Roland V31 drum module** — choosing kits, reading and
editing settings, and building custom kits — **using their phone's screen reader
and speech, without ever needing to see the module's screen.**

The Roland V31 (sold in the TD-313 / TD-316 / VAD316 kits) is a 2025 module with
a colour LCD and **zero built-in accessibility**: every kit selection, menu, and
sound edit assumes you can see the screen. The vendor ships nothing for blind
players. This app closes that gap.

---

## 2. The problem

- A blind drummer can *play* the V31 fine — pads are physical. But everything
  *around* playing requires sight: which kit is loaded, what the tempo is,
  changing instruments, adjusting trigger sensitivity, naming and saving kits.
- The module communicates over MIDI, but **it does not expose its screen, cursor,
  menu state, or navigation over MIDI** — only its *data model* (the parameter
  values). So we cannot "read the screen aloud."
- Therefore the only viable strategy is architectural (next section).

---

## 3. Core architectural decision: replace the screen, don't mirror it

**We do not try to reproduce the module's on-screen UI.** That is impossible: the
UI/cursor/menu state is not in the MIDI model. Instead:

- **The phone becomes the accessible interface** — a native, screen-reader-first
  app that speaks state and accepts edits.
- **The module becomes a headless sound engine + physical surface** — it makes
  sound and provides the pads; the phone drives its settings.
- **The link is MIDI System Exclusive (SysEx)**, fully documented in *V31 MIDI
  Implementation v2.00*. We read parameters with RQ1, write them with DT1, and
  receive live edits the module pushes when *Transmit Edit Data* is on.

See [ADR-0001](adr/0001-replace-not-mirror.md).

**Device-agnostic across Roland modules.** We start with the Roland V31, but the
app is built for **multiple Roland modules** (V51/V71 and future, unreleased ones).
Roland's SysEx mechanics are shared across modules; only the data differs (Model ID,
address map, catalogs, capabilities). So everything module-specific is a
**`DeviceProfile`** (data), auto-selected from the device's Identity Reply, and the
code stays generic — never hardcode V31. See [ADR-0007](adr/0007-device-profile-abstraction.md)
and [DEVELOPMENT.md §3](DEVELOPMENT.md#3-мульти-девайс-профили-устройств).

> This is **not** vendor- or instrument-agnostic *yet*. The longer-term vision —
> nonvisual interfaces for any electronic instrument, any vendor, with
> server-delivered profiles — is a deliberately deferred north star with cheap
> seams kept open. See [ADR-0012](adr/0012-scope-and-generalization-path.md).

---

## 4. Users and core principles

**Primary user:** a blind or low-vision drummer who already uses a smartphone
with a screen reader (VoiceOver on iOS, TalkBack on Android) and is fluent in its
gestures. They may run TTS at a very high speech rate. Hands are often occupied
with sticks. **This user is who we build for first** (see the
[North Star](#-north-star--nonvisual-first)).

**Secondary users:** sighted teachers/parents/techs helping set up; low-vision
players who can use large-text + high-contrast visuals alongside audio. They are
served by the **visual enhancement layer**, which is added on top of — and never
required by — the nonvisual experience.

Non-negotiable principles (full detail in [ACCESSIBILITY.md](ACCESSIBILITY.md)):

1. **Nonvisual-first (the North Star).** The nonvisual experience *is* the
   product. Build, test, and ship it first; the visual UI is a removable
   enhancement, never a requirement. A feature isn't done until it's usable
   eyes-closed. See the [North Star](#-north-star--nonvisual-first) and
   [ADR-0006](adr/0006-nonvisual-first.md).
2. **Fully native UI per platform, screen-reader-native, not a custom UI.** The
   UI is built **natively on each platform** — **SwiftUI on iOS, Jetpack Compose
   on Android** — using standard platform controls so VoiceOver/TalkBack work
   perfectly out of the box. **No cross-platform UI framework** (Flutter / React
   Native): their accessibility support is leakier and lags the OS, which is
   unacceptable when the screen reader *is* the product (§6.3, [ADR-0005](adr/0005-native-ui-per-platform.md)).
   Never invent a bespoke gesture language the user must learn.
3. **Everything is available non-visually.** No information conveyed by colour,
   position, or icon alone. Every state has a text/audio equivalent.
4. **No blind writes.** For a blind user, an edit that *might* have happened is
   worse than useless. Every edit follows **write → read back → verify → speak
   the actual stored value** (§10).
5. **The app talks back.** State changes — including ones the user causes on the
   hardware itself — are announced (§11), with throttling so speech never piles
   up while playing.
6. **Respect the platform.** Honour the user's system voice, speech rate,
   Dynamic Type / font scale, reduce-motion, and high-contrast settings.

---

## 5. Input strategy — the headline decision

> The user explicitly asked: **voice or gestures?** Short answer: **the framing
> is a false choice. Neither is the *primary* modality — the platform screen
> reader is.** Full reasoning in [ACCESSIBILITY.md §Input](ACCESSIBILITY.md#input-modality--the-real-answer)
> and [ADR-0003](adr/0003-input-screen-reader-first.md). Summary:

| Layer | Modality | When | Status |
|-------|----------|------|--------|
| **Primary** | Native accessible UI + OS screen reader (VoiceOver/TalkBack) — the standard swipe/double-tap gestures the user **already knows** | All setup & editing (done while *not* playing) | MVP |
| **Output (always on)** | Screen-reader announcements + non-speech earcons + haptics | Every state change, live | MVP |
| **Secondary** | **Performance mode**: a few huge, full-width targets (e.g. "next kit") that need no aiming | Quick changes between songs, hands briefly free | V1 |
| **Optional** | **Voice commands** (push-to-talk, small grammar, on-device) | Hands-free quick actions | V1+, behind expectations |
| **Future** | Hardware triggers (footswitch / dedicated pad / external MIDI controller) as a physical "next kit" surface | While playing | Exploratory |

**Why not "voice-first":** drumming is *loud*. Speech recognition degrades
badly in noise and the mic picks up the kit itself; voice is least reliable
exactly when the player is at the kit. It's a convenience layer, not a foundation.

**Why not "a custom gesture system":** blind users have deep muscle memory for
their screen reader. A bespoke gesture vocabulary would force them to relearn
interaction *and* would fight the screen reader. The win is to be *boringly,
perfectly compatible* with VoiceOver/TalkBack — which we get by using semantic
native controls. That is the "gestures" answer, and it's free.

---

## 6. System architecture

```
┌──────────────────────────────────────────────────────────┐
│  iOS app (Swift/SwiftUI)        Android app (Kotlin/Compose)│
│  • Accessible UI (VoiceOver)    • Accessible UI (TalkBack)  │
│  • CoreMIDI transport           • android.media.midi xport  │
│  • VoiceOver announcements      • TalkBack announcements    │
│  • Haptics / earcons            • Haptics / earcons         │
└───────────────┬───────────────────────────┬───────────────┘
                │   UniFFI bindings (Swift / Kotlin)          │
        ┌───────┴─────────────────────────────────────┐
        │  Rust core  (platform-agnostic, NO I/O)      │
        │  • SysEx codec: RQ1/DT1, checksum, 7-bit     │
        │    4-byte address arithmetic, nibble pack    │
        │  • Typed parameter model + address map       │
        │  • value ⇄ human string (to read)            │
        │  • Kit model, edit commands                  │
        │  • write→readback→verify state machine       │
        │  • emits "MidiOut bytes" / consumes "MidiIn" │
        └──────────────────────────────────────────────┘
                        │ embedded data (JSON)
        ┌───────────────┴──────────────┐
        │ Parameter map + instrument/FX │  ← generated from V31 Data List
        │ catalog (our derived JSON)    │     (build step, see §13)
        └───────────────────────────────┘
```

**Why this split:** the hard, bug-prone, testable logic (byte twiddling,
checksums, address math, value formatting, the verify state machine) lives **once**
in Rust with unit tests, and is shared by both platforms. The native layers stay
**thin**: move bytes, post announcements to the screen reader, render accessible
widgets. See [ADR-0002](adr/0002-rust-core-uniffi.md).

### 6.1 Core boundaries (important)

- The Rust core is **pure / no I/O**. It never opens a MIDI port or speaks. It
  takes inbound MIDI bytes + user intents, and returns outbound MIDI bytes +
  view-model updates (including the exact strings to announce). This makes it fully
  deterministic and unit-testable, and keeps platform code trivial.
- Transport and **screen-reader announcements** are **platform responsibilities**
  behind narrow interfaces — the app has no text-to-speech of its own
  ([ADR-0014](adr/0014-screen-reader-is-the-only-voice.md)).

### 6.2 Transport

- **USB-C MIDI = primary** (reliable, low-latency, simple permissions).
- **BLE-MIDI = secondary** (worse permissions/stability; nice-to-have).
- Device ID default `10h`; Model ID `01 06 01`. Identity handshake on connect.

### 6.3 UI layer — fully native per platform (decision)

**Decided:** the UI is **fully native on each platform**; only the logic is
shared (the Rust core, §6.1). No cross-platform UI framework.

- **iOS:** Swift + **SwiftUI** (drop to **UIKit** for any specific control where
  SwiftUI's accessibility falls short — pragmatic escape hatch, not the default).
- **Android:** Kotlin + **Jetpack Compose** (drop to Android Views likewise if a
  control's accessibility needs it).
- **Shared:** the Rust core via UniFFI — SysEx, parameter model, value⇄speech
  strings, the write→readback→verify machine. The two apps share *behaviour and
  data*, not screens.

**Why native, not Flutter / React Native / KMP-with-shared-UI:**

1. **Accessibility is the product.** Native platform widgets give the most
   complete, correct, and up-to-date VoiceOver/TalkBack behaviour (roles, the
   adjustable/rotor patterns, announcements, focus). Cross-platform UI toolkits
   historically expose a partial, lagging accessibility surface — a dealbreaker
   here.
2. **Right idioms per platform.** VoiceOver and TalkBack differ (gestures, rotor
   vs reading controls, announcement APIs). Native lets each app feel correct to
   users already fluent in *their* screen reader.
3. **Direct access to platform APIs** we need anyway: CoreMIDI / `android.media.midi`,
   the screen-reader **announcement** APIs (UIAccessibility / `AccessibilityManager`),
   haptics, accessibility-status detection.
4. **Cost is contained** because the hard, shared logic already lives in the Rust
   core — the native layers are thin (transport + announcements + accessible views).

**Trade-off accepted:** two UI codebases to maintain. Mitigated by the thin-UI /
shared-core split and a shared design spec so the two stay consistent.

**Future (deferred):** per-device UI becomes **downloadable data** — a declarative
description rendered by a **generic native renderer** per platform (you can't ship
downloadable native code; WebView would hurt a11y). Native renderers keep the
accessibility this decision is about. The core exposes a generic declarative
view-model to make this additive, not a rewrite. See
[ADR-0013](adr/0013-data-driven-ui-renderer.md).

---

## 7. Domain model

Mirrors the V31 address map (see [PROTOCOL.md](PROTOCOL.md)). Top-level areas:

- **Current** (`00 00 00 00`) — pointer to the active kit (`KitNum`, 0–199).
  Polled to know "which kit am I on" reliably (Program Change is not reliable —
  §8 / PROTOCOL).
- **Setup** (`01 00 00 00`) — global: Output routing, Control, Click/metronome,
  Misc.
- **Trigger** (`02 00 00 00`, banks 1–16) — per-pad trigger type & sensitivity.
- **SetList** (`03 00 00 00`, 1–32) — set lists.
- **Kit** (`04 00 00 00`, step `00 04 00 00`, kits 1–200; kit 200 = `0A 1C 00 00`):
  - `KitCommon` — **Kit Name** (16 ASCII bytes at offset 0), Sub Name, **Kit
    Tempo** (offset `00 6F`, value 200–2600 = 20.0–260.0 BPM), tempo switch, etc.
  - Per-pad units (`KitUnitCommon` / `KitUnitLayer`) for 28 pad/zone slots
    (KICK, SNARE HEAD/RIM, TOMs, HI-HAT, CRASHes, RIDE, AUX…): instrument
    selection per layer, pitch/decay/transient, volume/pan, pad EQ/comp, sends.
  - `KitFx` — 4 bus-FX slots (BUS-A/B × FX1/2).
  - Ambience — room / overhead / reverb / resonance.

The core exposes these as **typed parameters** with: address, encoding
(plain / nibble-packed / signed / ASCII), valid range, and a `format(value) →
string` for TTS (e.g. tempo `1200 → "120.0 BPM"`, pan `0 → "center"`).

---

## 8. Protocol integration (summary)

Full reference and **golden test vectors** in [PROTOCOL.md](PROTOCOL.md). Key
points that shape the app:

- **Read:** RQ1 `F0 41 dev 01 06 01 11 <addr×4> <size×4> sum F7` → module replies
  DT1.
- **Write:** DT1 `F0 41 dev 01 06 01 12 <addr×4> <data…> sum F7`.
- **Checksum** = `(128 − (Σ(addr+data) mod 128)) mod 128`. Two worked examples
  from the spec are encoded as unit tests (PROTOCOL §Golden vectors).
- **Live edits:** with **Transmit Edit Data = ON**, the module pushes DT1 when a
  knob is turned on the hardware → we parse address+value → announce it. This is
  how the app stays in sync with physical edits.
- **Current kit:** Program Change on kit change is **unreliable** (≤128, depends
  on PROG CHG mapping) → **poll `Current`** as the source of truth; treat any
  inbound PC as a hint to re-poll.
- **Firmware version:** the **Identity Reply** carries a 4-byte firmware version.
  Each device profile records the firmware it was tested against; an **untested
  firmware is announced at connect but never blocks** the app
  ([ADR-0009](adr/0009-firmware-compatibility-policy.md), [DEVELOPMENT §3.4](DEVELOPMENT.md#34-совместимость-прошивки-firmware)).

---

## 9. Feature scope

### 9.1 PoC (Web MIDI, throwaway — de-risk before mobile)
- Chrome + Web MIDI + USB-C. Prove RQ1→DT1 round-trip.
- Read Current kit + kit name + tempo; switch kit; **answer the persistence
  question** (does a DT1 write to a kit address survive a power cycle, or is a
  separate store action required?). This is **risk #1** (§14).

### 9.2 MVP
- Connect over USB-C MIDI; Identity handshake; clear connect/disconnect audio.
- Speak **current kit (number + name)** and **tempo**; update live (poll Current).
- **Switch kit** from an accessible list (write KitNum to Current, or PC).
- Resolve persistence so MVP edits are honest about what's saved.

### 9.3 V1
- **Global settings editor** (`Setup`: outputs, metronome/click, master, EQ).
- **Per-pad trigger/sensitivity editor** (`Trigger`).
- **Full kit editor:** rename; choose instrument per pad/layer (needs the
  instrument catalog from Data List); pitch/decay/transient; volume/pan; pad
  EQ/comp; sends.
- **FX:** choose effect *type* (the named effects) + presets — **not** the raw
  numeric parameters.
- **Ambience:** room / overhead / reverb / resonance — levels and types.
- **Live announce** of hardware edits via Transmit Edit Data.
- **Performance mode** (§5): big no-aim targets for next/previous kit.
- Every edit uses **write → readback → verify → speak** (§10).

### 9.4 Out of scope / not possible
- Mirroring the module's screen text, cursor, or menu navigation (not in MIDI).
- Blind tweaking of individual raw FX parameters (generic `−20000..20000` whose
  meaning depends on effect type — opaque and unsafe to edit non-visually). We
  expose curated types + presets instead.

---

## 10. The write → readback → verify state machine

For a blind user, "I think I changed it" is unacceptable. Every parameter edit
runs this cycle in the core:

```
Idle
 └─(user sets P = v)→ Writing      : send DT1(addr(P), encode(v))
        └────────────→ Reading     : send RQ1(addr(P), size(P))
              └──────→ Verifying    : on DT1 reply, decode actual a
                    ├─ a == v   → Confirmed : speak "<P name> <format(a)>"
                    ├─ a != v   → Mismatch  : speak "couldn't set <P>, it's still
                    │                          <format(a)>"  (announce truth, not intent)
                    └─ timeout  → Failed    : speak "no response, value unknown —
                                              check connection"
```

- The UI value the user hears is **always the read-back value**, never the
  optimistic local one.
- Edits are **serialized per parameter**; rapid changes coalesce (debounce) so we
  don't flood the module or the ear.
- Confirmation latency budget: see §12.

---

## 11. Audio output design (TTS + earcons + haptics)

Output is the heart of this app and the trickiest a11y problem, because the app
must surface **even when the user isn't touching the screen** (live hardware
edits, kit changes) — through the screen reader's announcement channel, never a
voice of our own.

- **Two feedback paths, one voice (the screen reader):**
  1. **UI-driven feedback** (user focused/activated a control): the screen reader
     already voices the focused control's value — we stay silent (no double-talk).
  2. **Live/ambient feedback** (kit changed on hardware, edit pushed): post an
     **accessibility announcement** so the screen reader voices it in the user's
     own voice — interrupting for navigation, coalescing duplicates.
- **Queue discipline:** never overlap announcements; newer high-priority messages
  (e.g. "kit changed") preempt stale low-priority ones; identical repeats are
  dropped.
- **Earcons (non-speech cues):** a short tone for fast events (connected,
  disconnected, kit changed, edit confirmed, error) so the player gets instant
  feedback *before* the slower spoken detail — crucial mid-performance.
- **Haptics:** confirm/deny patterns as a silent third channel.
- **Performance mode** biases toward earcons + terse speech ("Kit 12, Jazz") to
  stay out of the way while playing.
- **All speech strings come from the Rust core's `format()`**, so phrasing is
  consistent and unit-tested.

Detailed rules and the screen-reader coexistence strategy live in
[ACCESSIBILITY.md](ACCESSIBILITY.md).

---

## 12. Non-functional requirements

- **Latency:** kit-change announcement < ~300 ms after detection; edit
  confirmation (write→readback→speak) target < ~500 ms over USB-C.
- **Reliability:** no silent failures — every action ends in a spoken success or
  a spoken, actionable error. Reconnect automatically; announce link state.
- **Offline:** fully functional with no network. On-device TTS and (if used)
  on-device speech recognition. No telemetry without explicit opt-in.
- **Battery/thermals:** polling `Current` at a modest cadence (e.g. 2–4 Hz),
  backing off when idle.
- **Robustness:** tolerate partial/garbled SysEx, unknown addresses, and the
  module being power-cycled mid-session.

---

## 13. Data pipeline (Data List → JSON)

- A **build-time parser** reads the *V31 Data List* PDF (kept locally, see
  [vendor README](vendor/README.md)) and emits our own JSON:
  - **Instrument catalog** — `No. | group | name | remarks` (parses cleanly from
    the "Instrument list" table; remarks flag brush/cross-stick/etc. compat).
  - **FX / ambience type catalog** — named types + preset lists.
  - **Drum-kit preset list.**
  - **Parameter map** — address, range, encoding, units (cross-checked against
    the MIDI Implementation address map).
- The JSON is **embedded in the core**. It is our **derived data**, safe to
  commit; we do **not** commit the PDF (§17).
- **Caveat:** the instrument catalog **grows** via Roland Cloud expansions, so the
  catalog must be **versioned and updatable**, not assumed static (risk §14).

---

## 14. Risks and open questions

1. **Persistence (highest).** Does writing a kit parameter via DT1 land in the
   permanent slot, or only a temporary edit buffer that's lost on power-cycle? Is
   a separate "store/WRITE" action required? The MIDI doc shows **no explicit
   store command** and a `SNAPSHOT SAVE` function key exists on the hardware —
   strongly implying we must **verify on real hardware in the PoC** before
   promising "saved." Until resolved, the app must clearly distinguish
   *"changed (live)"* from *"saved to slot."*
2. **Instrument catalog drift** — expansions change the number↔name mapping →
   need an updatable, versioned catalog.
3. **BLE-MIDI** permissions/stability worse than USB-C → USB-C primary.
4. **Rust↔mobile binding** — UniFFI (preferred) vs hand-rolled C ABI + cbindgen.
5. **TTS/screen-reader coexistence** — latency and not stepping on VoiceOver/
   TalkBack while also speaking live events (§11).
6. **Voice recognition in noise** — confirms voice is a convenience layer, not a
   foundation (§5).
7. <a name="naming"></a>**Naming** — ✅ resolved: the project is **Tactus**
   (device-agnostic; avoids implying Roland affiliation, see NOTICE). The repo
   directory rename (`v31-vision` → `tactus`) is cosmetic and deferred.

---

## 15. Milestones

See [ROADMAP.md](../ROADMAP.md) for the phased plan. Order:
**PoC (de-risk persistence + round-trip) → Rust core + golden-vector tests →
Data List parser → one platform (Android: MIDI + TTS) end-to-end → second
platform → V1 editors.**

---

## 16. Testing strategy

**Nonvisual-first applies to tests too:** the *primary* acceptance tests assert
the **nonvisual** experience (correct labels, values, roles, announcements, and
the spoken state machine). They are written **first** and gate the feature.
Visual checks (snapshots, layout) are **secondary** and never substitute for the
nonvisual assertions. A feature with passing visual tests but missing/failing
nonvisual tests is **not** done (see [North Star](#-north-star--nonvisual-first)).

- **Core unit tests:** checksum + address arithmetic + nibble pack/unpack against
  the **golden vectors** in PROTOCOL.md; value⇄string round-trips for every
  parameter type (the exact spoken strings are asserted); the
  write→readback→verify state machine (with a simulated module).
- **Codec fuzzing:** malformed/partial SysEx must never panic.
- **Nonvisual UI tests (primary):** assert every control's label/value/role and
  every dynamic announcement, driven the way a screen reader would — XCUITest
  accessibility audit + element queries (iOS), Espresso/Compose semantics
  assertions + Accessibility Scanner (Android).
- **Manual eyes-closed pass (release gate):** operate the whole app with
  VoiceOver *and* TalkBack on, screen off/ignored — and, before release, sessions
  with **blind drummers**. Automated checks can pass while the app is unusable;
  human nonvisual testing is required (see ACCESSIBILITY.md).
- **Visual tests (secondary):** snapshot/layout checks for the enhancement layer,
  including largest Dynamic Type and high-contrast.
- **Hardware-in-the-loop:** a small harness that talks to a real V31 for
  round-trip, persistence, and live-edit tests.

---

## 17. Licensing & vendor documents

- Project license: **Apache-2.0** (patent grant is sensible for code implementing
  a third-party protocol; the `NOTICE` file carries the trademark/affiliation
  disclaimer). See `LICENSE`, `NOTICE`.
- **Roland's PDFs are © Roland and are NOT committed** — they're git-ignored in
  `docs/vendor/`. We ship only our own derived JSON/docs and cite the source.
  Rationale and download instructions: [vendor README](vendor/README.md),
  [ADR-0004](adr/0004-vendor-docs-not-committed.md).

---

## 18. Glossary

- **SysEx** — MIDI System Exclusive; vendor-specific messages (here: read/write
  V31 parameters).
- **RQ1 / DT1** — Roland "Data Request 1" (read) / "Data Set 1" (write/reply).
- **Kit** — a complete set of drum sounds + settings; the V31 holds 200.
- **Current** — the module's pointer to the active kit number.
- **Transmit Edit Data** — module setting that pushes parameter changes over MIDI
  as you edit on the hardware.
- **Screen reader** — VoiceOver (iOS) / TalkBack (Android); the OS service that
  speaks the UI and defines the gesture set blind users rely on.
- **Earcon** — a short non-speech audio cue with a learned meaning.
