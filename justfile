# V31 Vision — build orchestration.
# Install once:  brew install just
# (iOS recipes also need: rustup + iOS targets + cargo-swift — see the
#  "Set up the iOS build toolchain" task.)

# List available recipes
default:
    @just --list

# Core: format check, lint, and test
test-core:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Auto-format the Rust code
fmt:
    cargo fmt

# Build the core (host target, debug)
build-core:
    cargo build

# --- iOS (not wired yet — see tasks #2 and #11) ---

# Generate UniFFI Swift/Kotlin bindings
gen-bindings:
    @echo "TODO: uniffi-bindgen (task #11 — Define the UniFFI public API / build)"

# Build the iOS XCFramework + Swift Package
build-ios:
    @echo "TODO: cargo swift package (task #11)"
