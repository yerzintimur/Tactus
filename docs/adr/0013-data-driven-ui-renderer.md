# ADR-0013: Per-device UI = downloadable declarative description + a generic native renderer

**Status:** Accepted · **Date:** 2026-06-08

## Context
The vision (ADR-0012) deepens: not only the **profile** is per-device, but also the
**UI**, and both should be **downloaded on demand** in the future. The app is native
per platform (iOS + Android now; Mac / PC / Linux later).

Hard constraint: ADR-0005 commits to **native UI** for the best VoiceOver/TalkBack
accessibility. You **cannot download native UI code** — iOS forbids downloading
executable code (App Store guideline 2.5.2) — and a **WebView**-delivered UI would
degrade accessibility, contradicting nonvisual-first / ADR-0005.

## Decision
The downloadable per-device UI is a **declarative UI _description_ (data)**, not
platform code. Each platform ships a **generic, native, accessible renderer** that
interprets the description into native controls (SwiftUI / Compose / desktop).

- **Downloadable per device (data):** a **"device pack"** = profile (parameters) +
  **UI description** (sections, control kinds, order, labels) + catalogs. Pure,
  versioned, server-deliverable data.
- **Built-in per platform (code):** one generic **profile-UI renderer** per
  platform. Never downloaded. This is the only platform-native UI code.
- **Accessibility is preserved — even strengthened:** a declarative description is a
  semantic tree (label / role / value / language per node) that maps directly to
  accessible native elements. For nonvisual-first this is a *better* fit than
  hand-coded UI, because semantics are forced into the data model.
- **Most of the UI derives from the parameter model:** the profile already declares
  each parameter (type / range / encoding / i18n). The renderer generates accessible
  controls from them; the UI description only adds grouping / order / control-kind
  hints.
- **The core exposes a generic, declarative view-model** — a tree of sections →
  controls → values → localized labels (with mixed-language spans, ADR-0011) — so
  any platform renderer + any device works without bespoke per-screen code.

## Now vs later
- **Now (MVP):** build the V31 screens **directly** (hand-coded native), but design
  the **FFI view-model to be generic / declarative-friendly** so it doesn't
  foreclose a renderer. Do **not** build the generic renderer or the pack format
  yet (YAGNI; one device is not enough to design it well).
- **Later:** a generic native renderer per platform + downloadable device packs
  (profile + UI description). Tracked as a future task.

## Consequences
- FFI (task #10): prefer a generic, declarative view-model over per-screen types
  where practical; keep `CurrentKitChanged`-style specifics minimal.
- The "device pack" = profile + UI description + catalogs, all data, all
  server-deliverable.
- Renderers are the only platform-native UI code; they stay thin and generic.
- Reconciles ADR-0005 (native, accessible) with ADR-0012 (data-driven, deferred):
  the *renderer* is native; the *description* is data.
