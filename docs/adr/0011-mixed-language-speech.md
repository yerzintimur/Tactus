# ADR-0011: Mixed-language speech — tag device-sourced text with its language

**Status:** Accepted · **Date:** 2026-06-08

## Context
Tactus speaks in the user's locale (e.g. Russian: "Кит 5: …"), but a lot of
device-sourced text — **kit names, instrument names** — is **English ASCII**
("Jazz Funk", "TM SL Maple K"). A single-voice TTS reading the whole sentence in
one language mispronounces the foreign part (a Russian voice mangling "Jazz
Funk"). This is the classic code-switching TTS problem and it affects an
accessibility-critical path.

The output channel is the user's **screen reader** (VoiceOver / TalkBack) — the app
has no TTS of its own (see [ACCESSIBILITY.md](../ACCESSIBILITY.md) §4).

Verified platform facts:
- iOS: `UIAccessibilitySpeechAttributeLanguage` (`.accessibilitySpeechLanguage`)
  tags a *range* of an `NSAttributedString` so VoiceOver pronounces it in a given
  BCP-47 language. (Apple docs.)
- Android: `android.text.style.LocaleSpan` tags a span's locale so TalkBack
  pronounces it correctly. (Verify TalkBack honouring on device.)

## Decision
- The core's localized output carries **language spans**: the sentence is in the
  app locale, and substrings that are **device content** (kit/instrument names)
  are marked with a **device-content language** — default **"en"** (Roland names
  are English), configurable later.
- Native layers apply per-segment language: build an attributed announcement or
  label and tag the foreign range — iOS `.accessibilitySpeechLanguage`, Android
  `LocaleSpan`. VoiceOver/TalkBack then pronounce each part correctly.

**How the spans are found.** A device-sourced argument is wrapped in private-use
markers (`U+E000`/`U+E001`) *before* Fluent formats the message; the rendered
string is then split on the markers, which are stripped. Searching the output for
the argument's text afterwards would break as soon as a translation reorders or
repeats it, and a name that happened to contain the marker cannot leak one.

## Consequences
- `Speech` carries `spans: [{ text, lang }]` alongside the flat `text`
  (concatenating spans == text); native uses spans to tag ranges, `text` for
  display. One span means one language and the platform tags nothing.
- Device-content language defaults to "en"; a per-user/per-profile override can be
  added later (we can't truly detect the language of a user-typed kit name).
- **Announcements are tagged; static labels are not.** SwiftUI exposes no
  per-element language attribute — Swift's accessibility attribute scope has
  priority, pitch and punctuation but no language, which lives only as an
  `NSAttributedString.Key`. Announcements can therefore be posted as an
  `NSAttributedString` carrying both priority and per-range language, while a
  control whose label is a kit name still reads in the interface language.
  Closing that gap needs a UIKit-backed representable; the announcement path was
  built first because that is where a localized sentence and an English name
  actually collide in one utterance.
- Builds on [ADR-0008](0008-sans-io-core-and-i18n.md) (i18n in the core).
