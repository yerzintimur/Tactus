# Tactus — Apple app (iPhone · iPad · Mac)

Native SwiftUI app — one multiplatform target for iOS, iPadOS, and macOS.
Consumes the shared Rust core through the generated Swift package
(`bindings/Tactus`, built by `just build-ios`, which bundles iOS + macOS slices).

> The directory is still named `ios/` for now; the target builds for all Apple
> platforms. iPad is covered by the iOS destination; macOS is a native build (not
> Catalyst), with small platform shims for screen-reader announcements and earcons.

> Nonvisual-first: every screen must be fully usable eyes-closed (VoiceOver
> announcements + earcons/haptics). The app has no TTS of its own — the screen
> reader is the only voice (ADR-0014). See
> [docs/ACCESSIBILITY.md](../../docs/ACCESSIBILITY.md).

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
just mac-run         # build + launch the macOS app (ad-hoc signed)
```

On Mac, the machine is a **native USB host**, so a V31 connected straight to it
(no adapter) appears under **MIDI (debug) → Rescan** — the easiest way to test
against real hardware. The iOS Simulator does **not** see host MIDI devices, so
simulator testing uses the canned identity reply only.

## Connecting a module (hardware requirements)

USB MIDI only for now (no Bluetooth MIDI yet). The Simulator has no MIDI
endpoints, so a real module needs a physical device.

**On the module:** set the USB MIDI driver to **GENERIC** (class-compliant) — not
"Vendor"; iOS ships no vendor driver. Connect via the module's USB **COMPUTER**
port.

**The iPhone/iPad must be the USB _host_.** USB is asymmetric — one host, one
device. The module is a device, so the iDevice has to act as host:

- **USB-C iPhone (15/16) or iPad** — a USB-C cable straight to the module's port
  (USB-C↔USB-C, or USB-C↔USB-B as the module requires). iOS negotiates host mode
  over USB-C; **no adapter needed**.
- **Lightning iPhone (≤ 14)** — requires an **Apple Lightning to USB Camera
  Adapter** (the *USB 3* version with a power input is recommended — MIDI gear can
  draw more than Lightning alone supplies). Chain: `iPhone → adapter → USB cable →
  module`. A plain USB-C↔Lightning **charge cable does _not_ work**: it keeps the
  phone a peripheral and never enters host mode, so the module never enumerates
  (Sources/Destinations stay empty). This is a USB-role limitation of Lightning,
  not a connector-shape one — USB-C does the role negotiation a passive Lightning
  cable can't.

**Verify the link:** open the app → **MIDI (debug)** → **Rescan MIDI**. The
module's port name should appear under Sources/Destinations (also logged as
`MIDI scan: …`, subsystem `app.tactus`). Once it appears the transport
auto-connects and identification runs.

**Signing note (free Apple ID):** a personal-team build must be trusted on the
device (Settings → General → VPN & Device Management) and expires after 7 days —
rebuild from Xcode to renew. A paid Developer Program account lasts a year.

## Tests

```sh
just ios-test            # unit + UI tests on the iPhone 17 simulator
just ios-test "iPhone 16e"
```

- **Unit** (`TactusAppTests`) pin the Swift↔Rust boundary (canned V31 identity
  reply → device surfaced, ready state) and the ADR-0014 announcement routing.
- **UI** (`TactusAppUITests`) run the **full pipeline against the simulated
  module** (`--simulated-device`): identify → poll → kit navigation → tempo edit,
  each confirmed by real read-backs from `VirtualDeviceHandle` — no hardware.
  Includes the **accessibility audit gate** (`performAccessibilityAudit`) on the
  ready-state shipping UI (`--uitest` hides the DEBUG developer controls).

### Simulated module (no hardware)

Dev bindings (`just build-ios`) include the core's simulated V31 (cargo feature
`simffi` → `VirtualDeviceHandle`). Launching the app with `--simulated-device`
(DEBUG) swaps CoreMIDI for `SimulatedTransport`: the whole write→read-back→verify
pipeline, kit navigation, and timing behaviour run against the same profile-driven
model the Rust e2e harness tests. Shipping builds use **`just build-ios-release`**,
which builds without `simffi` and asserts the sim's symbols are absent.

The audit enforces the actionable, deterministic checks — text clipping, missing
labels, traits, hit regions, element detection. `contrast` and `dynamicType` are
**deliberately excluded for now**: standard iOS tinted controls (systemBlue ≈
3–4:1) fall below WCAG 4.5:1 and the audit flags them non-deterministically, so
they're unfit for a strict gate. A dedicated low-vision pass (high-contrast theme
+ Dynamic Type hardening) will re-enable them.

## Layout

```
apps/ios/
├── project.yml            # XcodeGen spec (committed)
├── TactusApp/
│   ├── TactusApp.swift     # @main App entry point (picks the transport at launch)
│   ├── CoreSession.swift   # bridge to the Rust core: drains Effects → @Published state
│   ├── ContentView.swift   # accessible MVP UI (connection, kit nav, rename, tempo)
│   ├── MidiTransporting.swift    # the transport seam CoreSession drives
│   ├── MidiTransport.swift       # CoreMIDI I/O (USB)
│   ├── SimulatedTransport.swift  # DEBUG: the core's simulated V31 (B1, --simulated-device)
│   ├── AnnouncementService.swift # posts to the screen reader's announcement channel (no TTS)
│   └── EarconService.swift # haptic earcons
├── TactusAppTests/         # unit tests (Swift↔Rust boundary)
└── TactusAppUITests/       # UI flow + accessibility audit gate
```

`CoreSession` is the one place that talks to the core. The core is sans-I/O: each
call returns `[Effect]` (send MIDI / schedule a tick / emit an event) that the app
performs — the injected `MidiTransporting` (CoreMIDI, or the simulated module
under `--simulated-device`) moves the MIDI, `AnnouncementService` posts
screen-reader announcements, and `EarconService` plays earcons/haptics. The DEBUG
**Developer** section can additionally drive the identity handshake with a canned
reply.
