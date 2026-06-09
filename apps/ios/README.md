# Tactus — iOS app

Native SwiftUI app. Consumes the shared Rust core through the generated Swift
package (`bindings/Tactus`, built by `just build-ios`).

> Nonvisual-first: every screen must be fully usable eyes-closed (VoiceOver +
> speech + earcons). See [docs/ACCESSIBILITY.md](../../docs/ACCESSIBILITY.md).

## Generated, not committed

Two things here are build artifacts (git-ignored, regenerated):

- `bindings/Tactus` — the core packaged as a Swift package (XCFramework + UniFFI
  bindings). Built by **`just build-ios`** (runs on the host; Apple's toolchain
  can't run in Docker).
- `Tactus.xcodeproj` — generated from **`project.yml`** by **XcodeGen**. Edit the
  spec, never the project.

What *is* committed: `project.yml` and the Swift sources under `TactusApp/`.

## First-time setup (host macOS)

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install 'cargo-swift@^0.11' just --locked
brew install xcodegen
```

## Build & run

```sh
just ios-bootstrap   # build the core package + generate the Xcode project
just ios-open        # open in Xcode (scheme: TactusApp)
# or, headless:
just ios-build       # build for the iOS Simulator
```

## Layout

```
apps/ios/
├── project.yml            # XcodeGen spec (committed)
└── TactusApp/
    ├── TactusApp.swift     # @main App entry point
    ├── CoreSession.swift   # bridge to the Rust core: drains Effects → @Published state
    └── ContentView.swift   # skeleton screen (real MVP UI: task #16)
```

`CoreSession` is the one place that talks to the core. The core is sans-I/O: each
call returns `[Effect]` (send MIDI / schedule a tick / emit an event) that the app
performs. The CoreMIDI transport (task #13) will inject `sendMidi` and feed inbound
bytes into `receive(_:)`; speech (`.speak`) routes to `AVSpeechSynthesizer` in task
#15. Until then, the **Developer** section drives the pipeline with a canned V31
identity reply.
