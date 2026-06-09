# ADR-0012: Scope now — Roland drums; vision — accessible interfaces for any instrument

**Status:** Accepted · **Date:** 2026-06-08

## Context
Tactus today targets **Roland drum modules** (V31 first). The maintainer's
longer-term vision is broader, on three axes:

1. **Vendor** — not only Roland.
2. **Instrument type** — not only drums, but *any* electronic musical instrument
   that can expose a controllable parameter interface (synths, grooveboxes, FX…).
3. **Delivery** — profiles *and their interface definitions* live on a **server**
   and are **downloaded on demand**, not only embedded in the app.

End state: Tactus is a **data-driven platform that delivers nonvisual (blind-first)
interfaces to electronic instruments** — effectively "a store of accessibility
profiles for hardware." This squarely serves the mission: equal, independent access
to instruments people already own.

## Decision
- **Scope stays Roland drum modules for now.** We do **not** build the vendor /
  instrument / server generalization yet — YAGNI, and a good abstraction needs ≥2
  real examples (we have one). See the self-audit reasoning that produced this.
- **But the architecture is kept positioned so each axis is a contained, additive
  step — never a rewrite:**
  - **Vendor** → introduce a `Protocol` trait (identity / parse / build-read /
    build-write); `sysex` becomes the *Roland* implementation; `engine` becomes
    generic over it. Today `engine` touches Roland only via ~4 `sysex::` calls.
  - **Instrument type** → the engine currently hardcodes drum-domain parameter ids
    (`current.kit_num`, `kit.common.tempo`). Generalizing means driving the engine
    from the **profile's own declaration** of what to read / poll / announce, so
    "kit/tempo" stops being special-cased. Profiles-as-data already point this way.
  - **Delivery** → `ProfileRegistry` already accepts `register()`ed profiles at
    runtime; embedded `with_builtin()` is just the offline bootstrap. Server packs
    = fetch JSON + register. The schema is versioned (`schema_version`).
- **Honesty in wording:** the accurate present-tense claim is "**device-agnostic
  across Roland modules**", not "vendor/instrument-agnostic". Docs corrected.

## Consequences
- **No code changes now**; the three seams are documented so future work is cheap.
- New code must not add fresh Roland/drum assumptions *beyond* the two known points
  (`sysex` = the Roland protocol; the engine's drum-domain parameter ids). Keep
  everything else (model i18n, the `Effect`/`CoreEvent` bus, the FFI, profiles)
  vendor- and instrument-neutral where practical.
- Profiles stay **pure, versioned data** so they are server-deliverable.
- Builds on [ADR-0007](0007-device-profile-abstraction.md) (profiles as data) and
  [ADR-0008](0008-sans-io-core-and-i18n.md) (sans-I/O core). Revisit and split into
  concrete ADRs (the `Protocol` trait; the generic instrument model; the profile
  server) when a second vendor or non-drum instrument becomes a real target.
