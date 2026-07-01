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
- Native layers apply per-segment language: build an attributed accessibility
  label and tag the foreign range — iOS `.accessibilitySpeechLanguage = "en-US"`,
  Android `LocaleSpan`. VoiceOver/TalkBack then pronounce each part correctly.
- **MVP** gets correct pronunciation purely via screen-reader element tagging
  (the kit-name label tagged "en") — **no core change required**. The core spans
  are the V1 path, applied when the accessibility labels are built.

## Consequences
- FFI `LocalizedText` carries `spans: [{ text, lang }]` alongside the flat `text`
  (concatenating spans == text); native uses spans to tag the accessibility
  label's ranges, `text` for display.
- The model's localizer gains a "device-content" arg marking + span output;
  implemented alongside its consumer (the iOS announcement/label layer) so we
  don't build speculative API ahead of need.
- Device-content language defaults to "en"; a per-user/per-profile override can be
  added later (we can't truly detect the language of a user-typed kit name).
- Builds on [ADR-0008](0008-sans-io-core-and-i18n.md) (i18n in the core).
