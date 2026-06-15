# ADR-0014: The screen reader is the only voice

**Status:** Accepted · **Date:** 2026-06-16

## Context
The app was emitting its **own** spoken announcements for nearly everything —
connection, kit changes, parameter reads, edit results — routed either through an
`AVSpeechSynthesizer` or, when VoiceOver is running, through accessibility
announcements ([ADR-0011](0011-mixed-language-speech.md), SPEC §9.3).

First live testing on the real V31 (2026-06-16, see [PROTOCOL §6](../PROTOCOL.md))
exposed two failures of that model, and the maintainer reframed the root cause:

1. **Speech flood.** Dialling through kits on the module pushed an unsolicited
   change per kit; the app read and spoke every name. A first attempt "fixed" this
   with a *debounce* (drop intermediate kits) — but that is wrong for a blind user:
   you **want** to hear each kit you land on. The correct mechanism is
   **interruption**, not suppression.
2. **Double speech.** Adjusting the tempo from the UI made VoiceOver voice the
   focused control's new value **and** the app announce the same value.

The deeper point: **a blind user already runs a configured screen reader**
(VoiceOver / TalkBack) — their chosen voice, rate, verbosity, and input (swipe,
rotor, voice control). A sighted user does not want TTS at all. An app that adds
its own voice is a *second*, competing narrator. It should not be one.

## Decision
**The user's screen reader is the single voice.** The app's job is a great
accessibility tree plus a thin announcement channel for what the screen reader
cannot observe on its own — never a second TTS over the top of it.

1. **Expose, don't narrate.** The app exposes a complete accessibility tree —
   labels, values, traits, adjustable actions. The screen reader voices
   everything it can observe: navigation and the value of the **focused** control.
   For these, the app stays **silent**.
2. **Announce only the screen-reader-invisible.** The app posts an accessibility
   **announcement** (through the screen reader's own announcement channel, *not* a
   separate synthesizer) only for changes the screen reader cannot see:
   - **device-initiated** changes — the user turned a knob / changed the kit on
     the **module** (an unsolicited DT1, [PROTOCOL §6](../PROTOCOL.md));
   - **connection** lifecycle (connected / disconnected / firmware note);
   - the **asynchronous result** of an action whose outcome the screen reader
     won't otherwise voice (e.g. an edit that the device **rejected**).
3. **Navigation interrupts (assertive).** A newer kit announcement **preempts**
   the previous one, so scrolling never queues a backlog. Slow scroll → you hear
   each kit; fast scroll → the interruptions leave you the one you settled on.
   This replaces the (removed) debounce.
4. **No double speech.** If the screen reader will voice a change — because the
   user just acted on a focused control — the app does **not** also announce it.
5. **Standalone TTS is optional.** The app's own `AVSpeechSynthesizer` /
   `TextToSpeech` path exists only for an explicit *"speak without a screen
   reader"* mode (a sighted drummer wanting audio cues, eyes/hands busy). It is
   **off by default** and is **never** used while a screen reader is running.

Earcons and haptics are **not** "voice" — they are unaffected by this ADR and
always play (they convey events the screen reader doesn't, without words).

## How the core and platform split the decision
The **sans-I/O core** ([ADR-0008](0008-sans-io-core-and-i18n.md)) cannot know
whether a screen reader is running or which control is focused. So:

- The **core** tags every spoken message with the semantics the platform needs:
  a **category** (`Connection` / `KitNav` / `ParamEdit` / `Error` / `Info`) and a
  **source** (`DeviceInitiated` vs `UserInitiated`). It still owns the localized
  text (one tested source of phrasing).
- The **platform** is the **router**. It knows the screen-reader state and focus,
  and decides *whether* and *how* to voice each message:
  - screen reader **off** → standalone TTS only if the user enabled it;
  - screen reader **on** + `UserInitiated` `ParamEdit` on the focused control →
    **suppress** (the screen reader voices the control's value);
  - `DeviceInitiated` or `Connection` → **announce**;
  - `KitNav` → **announce, interrupting**.

## Edge cases (resolved)
- **Async edit on a focused adjustable (tempo).** The edit is
  write→read-back→verify ([non-negotiable #3](../../AGENTS.md)); the verified value
  arrives ~a round-trip later, so the screen reader's *synchronous* post-gesture
  read would otherwise voice the **stale** value. Resolution: the control reflects
  the edit as **in-progress** until the device confirms; the screen reader voices
  the **device-verified** value (or, on mismatch, the app announces the actual
  value as a `DeviceInitiated` correction). The device stays the source of truth
  ([ADR-0010](0010-device-instances-and-source-of-truth.md)); any divergence is
  always spoken. The provisional in-progress state is **not** a blind write — it
  is never reported as the confirmed value, and the truth always wins audibly.
- **Connect vs first kit.** On connect the current kit is part of the **connection
  summary**, voiced once — not a `KitNav` barge-in that clobbers the connect line.
- **Tempo/param reads while scrolling.** Covered by interruption + the
  device-as-truth gating already in the engine: stale read-backs for a kit we have
  scrolled past are dropped from *content*, while the *announcement* of the settled
  kit interrupts any in-flight one.

## Consequences
- `SpeechService` becomes an **announcement router**, not a narrator: it
  suppresses app speech the screen reader already covers, posts interrupting
  announcements for navigation, and gates standalone TTS behind a setting.
- The core's `Speak` gains `category` + `source`; the FFI surface grows two enums.
- **Testing is via the real screen reader** (VoiceOver / TalkBack) — the authentic
  eyes-closed path ([ADR-0006](0006-nonvisual-first.md)) — not the app's TTS.
- Refines [ADR-0003](0003-input-screen-reader-first.md) (screen-reader-first input)
  and [ADR-0006](0006-nonvisual-first.md); supersedes the "app always speaks"
  reading of SPEC §9.3 and [ADR-0011](0011-mixed-language-speech.md) (per-segment
  language tagging still applies **to** announcements, it just isn't a second TTS).
