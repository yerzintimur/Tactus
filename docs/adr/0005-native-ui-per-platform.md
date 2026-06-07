# ADR-0005: Fully native UI per platform (SwiftUI + Jetpack Compose), no cross-platform UI framework

**Status:** Accepted · **Date:** 2026-06-07

## Context
We ship on both iOS and Android. The shared, risky logic already lives in the
Rust core ([ADR-0002](0002-rust-core-uniffi.md)), so the open question was only
about the **UI layer**: build it once with a cross-platform toolkit (Flutter,
React Native, or Kotlin Multiplatform with shared Compose UI), or build it
**natively per platform**. Because this app exists *for* screen-reader users, the
quality of the accessibility layer is not a feature — it is the entire product.

## Decision
Build the UI **fully natively on each platform**:
- **iOS:** Swift + **SwiftUI** (fall back to **UIKit** for any individual control
  where SwiftUI's accessibility is insufficient — a deliberate escape hatch, not
  the default).
- **Android:** Kotlin + **Jetpack Compose** (fall back to Android Views likewise).
- **Shared:** only the Rust core via UniFFI (SysEx codec, parameter model,
  value⇄speech strings, write→readback→verify). The apps share behaviour and
  data, not screens.

## Alternatives considered
- **Flutter** — single UI codebase, but accessibility is a re-implemented layer
  on a custom rendering engine; it trails native VoiceOver/TalkBack behaviour and
  has a history of a11y gaps. Rejected.
- **React Native** — maps to native views, better than Flutter for a11y, but still
  an abstraction with leaks and version lag on accessibility APIs. Rejected.
- **Kotlin Multiplatform + shared Compose UI** — Compose Multiplatform on iOS does
  not give first-class UIKit/VoiceOver semantics. Rejected for the UI (KMP for
  shared *logic* is moot since we use Rust).

## Rationale
1. **Accessibility is the product.** Native widgets give the most complete,
   correct, up-to-date screen-reader behaviour (roles, adjustable/rotor patterns,
   announcements, focus order).
2. **Platform-correct idioms.** VoiceOver and TalkBack differ; native lets each
   app feel right to users fluent in their own screen reader.
3. **Direct platform APIs** we need anyway (CoreMIDI / `android.media.midi`, TTS,
   haptics, accessibility-status detection).
4. **Contained cost** — the hard shared logic is in Rust, so native UI layers stay
   thin.

## Consequences
- Two UI codebases to build and maintain (the explicit cost).
- Mitigated by the thin-UI / shared-core split and a shared design/interaction
  spec so iOS and Android stay consistent.
- Every UI control is verified with the real platform screen reader
  (see [ACCESSIBILITY.md](../ACCESSIBILITY.md)).
