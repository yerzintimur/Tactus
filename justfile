# Tactus — build orchestration.
# Core builds/tests/lints run IN DOCKER (reproducible). The iOS leg runs on the
# HOST (macOS + Xcode) — Apple's toolchain can't run in Docker.
# Install once:  brew install just   (the cargo-docker.sh script also works directly)

# List available recipes
default:
    @just --list

# (Re)build the pinned core Docker image
docker-image:
    docker build -t tactus-core:1.93.0 -f docker/Dockerfile .

# Core: format check + lint + test — in Docker
# (the second clippy/test pass covers the feature-gated sim FFI, which the
# workspace-wide default-features run would otherwise never compile)
test-core:
    ./scripts/cargo-docker.sh fmt --check
    ./scripts/cargo-docker.sh clippy --all-targets -- -D warnings
    ./scripts/cargo-docker.sh clippy -p tactus --features simffi --all-targets -- -D warnings
    ./scripts/cargo-docker.sh test
    ./scripts/cargo-docker.sh test -p tactus --features simffi

# End-to-end tests: the engine driven against the virtual device + cassette replay.
test-e2e:
    ./scripts/cargo-docker.sh test -p devicesim -p e2e

# Auto-format — in Docker
fmt:
    ./scripts/cargo-docker.sh fmt

# Build the core (debug) — in Docker
build-core:
    ./scripts/cargo-docker.sh build

# Arbitrary cargo in the pinned image, e.g.  just cargo test -p sysex
cargo *ARGS:
    ./scripts/cargo-docker.sh {{ARGS}}

# --- iOS (HOST macOS + Xcode — NOT Docker) ---
# Apple's toolchain can't run in Docker. One-time host setup:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
#   cargo install 'cargo-swift@^0.11' just --locked
# The generated package (bindings/Tactus) is git-ignored and regenerated.

# Regenerate the Swift package: builds libtactus.a for iOS (device + simulator)
# and macOS, generates the UniFFI Swift bindings, and bundles the XCFramework.
# arm64 only (we ship arm64); Intel slices (iOS-sim + macOS) are excluded.
# DEV bindings include the simulated module (`simffi` → VirtualDeviceHandle) so
# the app + UI tests can run the full pipeline with no hardware (`--simulated-device`).
# Shipping builds must use `build-ios-release` instead (no sim inside).
build-ios features="simffi":
    rm -rf core/crates/ffi/Tactus
    cd core/crates/ffi && cargo swift package \
        --platforms ios@18 macos@14 --name Tactus --release \
        --lib-type static --accept-all --skip-toolchains-check \
        --exclude-arch x86_64-apple-ios --exclude-arch x86_64-apple-darwin \
        {{ if features == "" { "" } else { "--features " + features } }}
    # cargo-swift exits 0 even on failure, so verify the artifact exists.
    test -f core/crates/ffi/Tactus/tactusFFI.xcframework/Info.plist
    rm -rf bindings/Tactus && mkdir -p bindings
    mv core/crates/ffi/Tactus bindings/Tactus
    # cargo-swift emits swift-tools-version:5.5, but .iOS(.v18) needs 6.0.
    sed -i '' '1s|.*|// swift-tools-version:6.0|' bindings/Tactus/Package.swift
    @echo "✅ bindings/Tactus regenerated (XCFramework + Swift bindings)"

# Shipping bindings: NO simulated device inside. Asserts the sim's API and
# symbols are absent from the generated Swift and every XCFramework slice.
# (The Debug app config references SimulatedTransport, so pair these bindings
# with Release builds only; rerun `just build-ios` to get dev bindings back.)
build-ios-release: (build-ios "")
    ! grep -rq "VirtualDeviceHandle" bindings/Tactus/Sources
    ! find bindings/Tactus/tactusFFI.xcframework -name '*.a' \
        -exec nm {} + 2>/dev/null | grep -qi virtualdevicehandle
    @echo "✅ release bindings clean (no simulated-device symbols)"

# cargo-swift fuses bindgen + packaging into one step.
gen-bindings: build-ios

# Generate apps/ios/Tactus.xcodeproj from project.yml (XcodeGen).
# One-time host setup:  brew install xcodegen
ios-gen:
    cd apps/ios && xcodegen generate

# Full iOS bootstrap: (re)build the core package, then (re)generate the project.
ios-bootstrap: build-ios ios-gen
    @echo "✅ open apps/ios/Tactus.xcodeproj in Xcode (scheme: TactusApp)"

# Build the iOS app for the simulator (headless verification).
ios-build:
    xcodebuild -project apps/ios/Tactus.xcodeproj -scheme TactusApp \
        -destination 'generic/platform=iOS Simulator' \
        -derivedDataPath .docker/ios-derived build

# Run the iOS unit + UI tests (incl. the accessibility audit gate) on a sim.
ios-test sim="iPhone 17":
    xcodebuild test -project apps/ios/Tactus.xcodeproj -scheme TactusApp \
        -destination 'platform=iOS Simulator,name={{sim}}' \
        -derivedDataPath .docker/ios-derived

# Open the project in Xcode.
ios-open:
    open apps/ios/Tactus.xcodeproj

# --- macOS (same multiplatform app target) ---
# Build the Mac app, ad-hoc signed so it runs locally without a dev team.
# (From Xcode, just pick "My Mac" + your team — Mac runs have no 7-day limit.)
mac-build:
    xcodebuild -project apps/ios/Tactus.xcodeproj -scheme TactusApp \
        -destination 'platform=macOS,arch=arm64' -derivedDataPath .docker/mac-derived \
        build CODE_SIGN_IDENTITY="-" CODE_SIGN_STYLE=Manual \
        CODE_SIGNING_REQUIRED=NO CODE_SIGNING_ALLOWED=YES

# Build + launch the Mac app. The Mac is a native USB host, so a V31 connected
# straight to it (no adapter) shows up under MIDI (debug) → Rescan.
mac-run: mac-build
    open .docker/mac-derived/Build/Products/Debug/TactusApp.app

# Build + launch the Mac app against the SIMULATED module (no hardware): the
# full pipeline incl. VoiceOver eyes-closed testing, powered by devicesim (B1).
mac-run-sim: mac-build
    open .docker/mac-derived/Build/Products/Debug/TactusApp.app --args --simulated-device
