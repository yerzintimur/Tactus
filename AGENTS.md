# AGENTS.md

Context file for AI agents **and** human contributors. Read this first. It is the
short, authoritative orientation; the deep design lives in [docs/](docs/).

---

## What this is

**Tactus** — an **accessibility-first companion app** for **blind and low-vision
drummers** who use Roland electronic drum modules. The phone becomes the accessible
interface (screen reader + speech + voice) and the module is driven over **MIDI
SysEx**. Targets **iOS + Android**, sharing a single **Rust core**. (Name origin &
mission: see [README](README.md).)

Built **iOS-first**, for the **Roland V31** for now. The architecture is
**device-agnostic _across Roland modules_** (see [Multi-device](#multi-device-architecture)):
V51/V71 and future Roland modules are added as **data (device profiles)**, ideally
with no code changes.

> **North star (deferred, not built yet):** Tactus is meant to grow into a
> **data-driven platform delivering nonvisual interfaces to _any_ electronic
> instrument** — beyond drums, beyond Roland — with **profiles served from a
> backend and downloaded on demand**. We deliberately keep that out of scope now
> but keep the seams cheap. So the precise claim today is "Roland-device-agnostic",
> **not** vendor/instrument-agnostic. See [ADR-0012](docs/adr/0012-scope-and-generalization-path.md).
>
> Not only the profile — the **per-device UI is data too**: a declarative
> description (downloadable per device) rendered by a **generic native renderer per
> platform** (you can't download native code, and WebView would hurt a11y). The
> core therefore exposes a **generic, declarative view-model**. MVP builds V31
> screens by hand, but the FFI view-model stays renderer-friendly. See
> [ADR-0013](docs/adr/0013-data-driven-ui-renderer.md).

> The repo directory is still `v31-vision` (cosmetic rename to `tactus` deferred).
> The shared core library builds as **`libtactus`** (FFI crate `tactus`). Keep code
> namespaces neutral/brand-based (`sysex`, `device`, `model`, `engine`, `tactus`),
> never `v31`.

---

## Non-negotiables (do not violate)

1. **Nonvisual-first.** The nonvisual experience *is* the product. Build, test, and
   ship every feature **eyes-closed first** (screen reader + speech + earcons +
   haptics); the visual UI is a **removable enhancement**, never required.
   *"If it can't be done eyes-closed, it isn't done."*
   → [North Star](docs/SPEC.md#-north-star--nonvisual-first), [ADR-0006](docs/adr/0006-nonvisual-first.md),
   gate in [CONTRIBUTING.md](CONTRIBUTING.md).
2. **No Roland documents in git.** Roland's PDFs are © Roland — git-ignored in
   `docs/vendor/`. Commit only our **own derived data/notes**, and cite the source.
   → [ADR-0004](docs/adr/0004-vendor-docs-not-committed.md).
3. **No blind writes.** Every parameter edit is **write → read back → verify →
   speak the actual stored value**. Never report intent as fact.

---

## Language policy

- **Everything committed to the repo is in English** — code, identifiers,
  comments, commit messages, public docs (README, CONTRIBUTING, ADRs, SPEC,
  PROTOCOL, ACCESSIBILITY), and this file.
- **The app itself is multilingual (i18n/l10n)** — all user-facing/spoken strings
  are localizable; default to the device locale, allow override. See the i18n
  design in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).
- **Working specs the maintainer requested in Russian are temporary.** Right now
  the deep development spec [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) is written in
  **Russian** for the maintainer's review. These Russian working specs will be
  **translated to English or removed before public release**. (The other docs in
  `docs/` are already in English — they are ahead of that curve.)
- **Collaboration language with the maintainer is Russian** (chat, planning,
  review). Deliverables that land in the repo follow the English rule above.

When in doubt: *repo artifact → English; conversation → Russian.*

---

## Multi-device architecture

The single most important architectural idea: **separate the invariant protocol
mechanics (code) from the per-device model (data).**

- **Roland SysEx mechanics are shared** across modules (RQ1/DT1 framing, checksum,
  4-byte 7-bit addresses, nibble/signed/ASCII encodings). This is **code**, in the
  `sysex` crate, and knows nothing about any specific module.
- **Everything that differs per module is a `DeviceProfile`** — Model ID,
  capabilities (pad layout, kit count, FX slots, feature flags), the parameter
  **address map**, and the **catalogs** (instruments / FX / ambience). This is
  **data** (JSON/RON under `profiles/`), versioned and updatable.
- On connect, the app sends an **Identity Request**; the **Identity Reply's Model
  ID** selects the matching profile (auto-detect). Unknown module → graceful
  generic/degraded mode + an invitation to contribute a profile.
- **Firmware:** the Identity Reply also carries the **4-byte firmware version**.
  Each profile lists the firmware it was **tested** against; an untested version is
  **announced at connect but never blocked** — the app keeps working
  ([ADR-0009](docs/adr/0009-firmware-compatibility-policy.md)).
- **Future modules** ship as a new profile (data), downloadable as a "profile
  pack" without an app update where possible.

Never hardcode V31 specifics in `core`, `apps/*`, or the FFI. If it varies by
module, it belongs in a profile.

---

## Monorepo layout

```
<repo>/
├── AGENTS.md                  # this file
├── README.md  CONTRIBUTING.md  ROADMAP.md  LICENSE  NOTICE
├── justfile                   # build orchestration (just build-ios / build-android / gen-bindings)
├── rust-toolchain.toml        # pins Rust + edition
├── Cargo.toml                 # Rust workspace
├── core/crates/
│   ├── sysex/                 # device-agnostic Roland SysEx codec + encodings (no I/O)
│   ├── device/                # DeviceProfile schema, registry, auto-detection, capabilities
│   ├── model/                 # domain model (kit/params/edits) + i18n value→speech (Fluent)
│   ├── engine/                # sans-I/O session FSM: connect, poll, write→readback→verify, events
│   └── ffi/                   # #[uniffi::export] public API (crate-type: cdylib + staticlib + lib)
├── profiles/                  # device profile DATA (our derived JSON) + schema; e.g. roland-v31.json
├── tools/                     # parsers (Data List PDF → profile JSON), codegen
├── apps/
│   ├── ios/                   # Xcode + SwiftUI; consumes core via SPM binaryTarget (XCFramework)
│   └── android/               # Gradle + Jetpack Compose; jniLibs from cargo-ndk
├── bindings/                  # generated Swift/Kotlin + built libs (GITIGNORED, regenerated)
├── poc/web-midi/              # throwaway TypeScript Web MIDI PoC
└── docs/                      # SPEC, ACCESSIBILITY, PROTOCOL, ADRs (EN) + DEVELOPMENT.md (RU, temp)
```

**Core is sans-I/O** (no MIDI port, no TTS, no timers). It receives events
(`handle_midi_input`, `tick`, user intents) and emits actions/events (send these
MIDI bytes, speak this localized message, schedule a tick). Native layers own all
I/O. This makes the core fully deterministic and unit-testable.

---

## Tech stack & pinned versions (mid-2026)

> **Always verify the latest before locking** (`rustc --version`, crates.io,
> developer.android.com, Apple developer). WWDC 2026 (Jun 8) → iOS 27 / Xcode 27
> betas imminent; stay on stable until GA (~Sept 2026).

| Area | Choice | Version (verify) |
|------|--------|------------------|
| Rust | toolchain, edition 2024 | 1.93.x (installed; pinned in `rust-toolchain.toml`) |
| FFI | **UniFFI** (proc-macro, library mode) | 0.31.x · keep `uniffi`/`uniffi-bindgen` identical |
| Android pkg | cargo-ndk + NDK (16 KB pages) | cargo-ndk 4.1.2 · NDK r28+ |
| Android FFI | JNA direct mapping (UniFFI default) | JNA ≥ 5.12.0 `@aar` |
| Android | Kotlin / AGP / Gradle / Compose BOM | 2.4.0 / 9.2.0 / 9.5.1 / 2026.06.00 |
| Android SDK | compileSdk / targetSdk / minSdk | 36 / 36 / 26 |
| Android ABIs | ship | `arm64-v8a`, `x86_64` |
| iOS pkg | XCFramework via cargo-swift | cargo-swift 0.11.x |
| iOS | Xcode / Swift / min iOS | 26.5 / 6.3 / iOS 18 |
| Build | orchestration | justfile |

See full rationale, alternatives, and exact build commands in
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

---

## Conventions

- **Builds run in Docker.** All Rust core builds/tests/lints — and later Android
  and tooling — run in the pinned image (`docker/Dockerfile`, Rust 1.93.0) via
  **`scripts/cargo-docker.sh`** (or `just test-core` / `just build-core`). Don't
  invoke host `cargo` for core work; keep the host clean and builds reproducible.
  Caches + build dir live in `.docker/` (git-ignored). **Exception — the iOS leg**
  (Xcode, XCFramework, the `*-apple-ios` targets, simulator, device) runs on
  **host macOS + Xcode**: Apple's toolchain cannot run in Docker.
- **Rust:** `cargo fmt` + `cargo clippy -D warnings`; release profile uses
  `lto`, `panic = "abort"`, `strip`, `opt-level = "z"`. Pure logic in `core`,
  thin platform code in `apps/*`.
- **Code layout (idiomatic Rust, not "one item per file"):** the *crates*
  (`sysex`/`device`/`model`/`engine`/`tactus`) are the architectural layers —
  the compiler enforces the dependency direction (lower crates can't see higher
  ones). *Inside* a crate, decompose by **cohesive module/concern** (`src/lib.rs`
  stays thin: module declarations + `pub use` re-exports + crate docs; each
  concern is its own `src/<concern>.rs`, e.g. `sysex` → `checksum.rs`,
  `message.rs`). Split when a file grows past one clear concern — **don't**
  over-shard into a file per function/type.
- **Testing (cover the maximum):**
  - **Unit tests** inline as `#[cfg(test)] mod tests` in the same file (can see
    private items).
  - **Integration tests** in `<crate>/tests/*.rs` pin the **public** contract
    (e.g. `sysex/tests/golden_vectors.rs`).
  - **Doc tests** on public functions (runnable examples).
  - **Property tests** (`proptest`) and **fuzz** (`cargo-fuzz`) for codecs/parsers
    (encodings, address math, SysEx reassembly).
  - Everything stays green: `cargo fmt --check`, `cargo clippy -D warnings`,
    `cargo test` (see `just test-core`); CI enforces this on every push.
- **Speech strings** come from the core (localized via Fluent), not hardcoded in
  the UI — single tested source of phrasing across platforms.
- **Anything device-specific → a profile, not code.**
- **Generated bindings & built libs are git-ignored** and regenerated; never
  commit stale ones. Pin generator and runtime to the same UniFFI version.
- **Commits:** Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:` …),
  English, imperative.
- Don't commit Roland PDFs or `*.local` settings.

---

## Key decisions (ADRs)

- [0001](docs/adr/0001-replace-not-mirror.md) Replace the module's screen, don't mirror it.
- [0002](docs/adr/0002-rust-core-uniffi.md) Shared Rust core + UniFFI, thin native layers.
- [0003](docs/adr/0003-input-screen-reader-first.md) Screen-reader-first input.
- [0004](docs/adr/0004-vendor-docs-not-committed.md) Roland docs not committed.
- [0005](docs/adr/0005-native-ui-per-platform.md) Fully native UI per platform.
- [0006](docs/adr/0006-nonvisual-first.md) Nonvisual-first philosophy.
- [0007](docs/adr/0007-device-profile-abstraction.md) Device-profile abstraction for multi-device support.
- [0008](docs/adr/0008-sans-io-core-and-i18n.md) Sans-I/O core + i18n (Fluent) in the core.
- [0009](docs/adr/0009-firmware-compatibility-policy.md) Firmware compatibility — detect, announce, never block.
- [0010](docs/adr/0010-device-instances-and-source-of-truth.md) Device instances vs types; device is the source of truth.
- [0011](docs/adr/0011-mixed-language-speech.md) Mixed-language speech — per-segment language tagging.
- [0012](docs/adr/0012-scope-and-generalization-path.md) Scope now = Roland drums; vision = any instrument/vendor, server-delivered profiles (deferred).
- [0013](docs/adr/0013-data-driven-ui-renderer.md) Per-device UI = downloadable declarative description + generic native renderer (deferred).

## Deep design

The very detailed development/architecture spec is **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)**
(currently Russian — see Language policy). It covers requirements, the device-profile
schema, every core crate, the UniFFI contract, i18n, the accessibility API mapping,
build/CI, versions, trade-offs, and the implementation order.
