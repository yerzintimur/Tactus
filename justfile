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

# --- iOS (HOST macOS + Xcode — NOT Docker; tasks #2, #11) ---
gen-bindings:
    @echo "TODO: uniffi-bindgen — runs on host (task #11)"
build-ios:
    @echo "TODO: cargo swift package — host macOS + Xcode, NOT Docker (task #11)"
