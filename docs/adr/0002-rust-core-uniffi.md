# ADR-0002: Shared Rust core, thin native layers, UniFFI bindings

**Status:** Accepted · **Date:** 2026-06-07

## Context
We target iOS and Android. The risky, detail-heavy logic — SysEx encoding,
checksums, 7-bit 4-byte address arithmetic, nibble/signed/ASCII value codecs,
value→speech formatting, and the write→readback→verify state machine — is
identical on both platforms and must be exactly correct (a blind user can't catch
a silent encoding bug).

## Decision
Put all of that logic in a **single platform-agnostic Rust core with no I/O**,
unit-tested against golden vectors, and expose it to Swift/Kotlin via **UniFFI**.
The native layers stay thin: MIDI transport, TTS, and accessible UI only. The
core consumes inbound MIDI bytes + user intents and returns outbound MIDI bytes +
view-model updates (including exact strings to speak).

## Alternatives considered
- **Hand-rolled C ABI + cbindgen** — more control, more boilerplate and unsafe
  glue. Kept as fallback if UniFFI limits us.
- **Duplicate logic per platform** — rejected: doubles the surface for subtle,
  hard-to-detect codec bugs.

## Consequences
- One tested implementation of the dangerous code; platforms are trivial.
- Build complexity: Rust toolchains + UniFFI in two mobile builds.
- Core is fully deterministic and testable without hardware (simulated module).
