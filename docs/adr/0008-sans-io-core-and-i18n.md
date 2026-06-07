# ADR-0008: Sans-I/O core, and localization (i18n) lives in the core

**Status:** Accepted · **Date:** 2026-06-07

This ADR records two tightly related core-design decisions.

## Context
The Rust core must be shared by iOS and Android, be exactly correct (a blind user
can't catch a silent bug), and produce the **spoken strings** that are the heart of
the product. The app must also be **multilingual**. Two questions:
1. How does the stateful session logic (connection, polling, the
   write→readback→verify cycle) run without making the core do I/O or own threads?
2. Where do localized, spoken strings come from?

## Decision

### A. Sans-I/O core
The core does **no I/O** and owns **no threads or timers**. The `engine` is a pure
state machine driven by inbound events — `on_connected`, `handle_midi_input(bytes)`,
`tick(now)`, and user intents — and it emits **actions/events**: *send these MIDI
bytes*, *speak this localized message with this priority*, *schedule a tick*, *here
is a view-model update*. The native layers own the MIDI port, the timer, and the
TTS; they feed events in and execute actions out.

The FFI exposes a `Session` object plus callback interfaces the native side
implements (`MidiSender`, `SessionListener`), so the API is ergonomic while the
engine underneath stays a deterministic, pure FSM.

### B. Localization in the core (Fluent)
The spoken output and parameter value formatting are **localized inside the core**
using **Fluent** (`fluent-bundle`) catalogs (`.ftl`) bundled per locale. `model`
returns `Message { id, args }`; the core renders the final `LocalizedText` for the
current locale (handling plural rules and number formatting). Pure UI-chrome
strings not tied to module data use **native resources** (Android string resources,
iOS String Catalogs).

## Rationale
- **Sans-I/O** gives a fully deterministic core: testable without hardware,
  threads, or flaky timing; trivial to simulate a module in unit tests; no data
  races across the FFI.
- **Localization in the core** keeps the project's promise that "all speech strings
  come from the core" — one tested source of phrasing, consistent across both
  platforms, with proper plural/number handling. The OS still provides the actual
  TTS voice.

## Alternatives considered
- **Async core with internal runtime/threads** — more "natural" calls but
  nondeterministic, harder to test, FFI threading hazards. Rejected as the base
  model (UniFFI async is still available where genuinely needed).
- **Localization only in native resources** (no i18n in core) — standard
  translation tooling per platform, but duplicates formatting logic and risks the
  two apps drifting in phrasing; weakens the "speech from core" guarantee.
  Rejected for spoken/data strings; still used for pure UI chrome.

## Consequences
- Native layers must drive `tick()` and feed transport events (a little extra
  wiring), documented in [DEVELOPMENT.md](../DEVELOPMENT.md).
- Translators edit `.ftl` files for spoken/data strings and platform resource
  files for UI chrome — document both in the contributor guide.
- Builds on [ADR-0002](0002-rust-core-uniffi.md) (shared Rust core + UniFFI).
