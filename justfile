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
test-core:
    ./scripts/cargo-docker.sh fmt --check
    ./scripts/cargo-docker.sh clippy --all-targets -- -D warnings
    ./scripts/cargo-docker.sh test

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

# Regenerate the Swift package: builds libtactus.a for device + simulator,
# generates the UniFFI Swift bindings, and bundles tactusFFI.xcframework.
# arm64 only (we ship arm64); the Intel-simulator slice is excluded.
build-ios:
    rm -rf core/crates/ffi/Tactus
    cd core/crates/ffi && cargo swift package \
        --platforms ios@18 --name Tactus --release \
        --lib-type static --accept-all --skip-toolchains-check \
        --exclude-arch x86_64-apple-ios
    # cargo-swift exits 0 even on failure, so verify the artifact exists.
    test -f core/crates/ffi/Tactus/tactusFFI.xcframework/Info.plist
    rm -rf bindings/Tactus && mkdir -p bindings
    mv core/crates/ffi/Tactus bindings/Tactus
    # cargo-swift emits swift-tools-version:5.5, but .iOS(.v18) needs 6.0.
    sed -i '' '1s|.*|// swift-tools-version:6.0|' bindings/Tactus/Package.swift
    @echo "✅ bindings/Tactus regenerated (XCFramework + Swift bindings)"

# cargo-swift fuses bindgen + packaging into one step.
gen-bindings: build-ios
