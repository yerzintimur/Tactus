# Pinned toolchain for reproducible Tactus **core** builds/tests/lints.
#
# Scope: Linux-able work only — the Rust core, its tests/lints, tooling, and
# (later) the Android cross-build. The iOS leg (Xcode, XCFramework, the
# *-apple-ios targets) CANNOT run here: the Apple toolchain is macOS-only. Run
# iOS builds on the host (macOS + Xcode).
#
# Source is NOT copied in — scripts/cargo-docker.sh bind-mounts the repo at /work
# and points CARGO_HOME / CARGO_TARGET_DIR / HOME under /work/.docker at runtime,
# so all writes stay out of git and owned by the host user.
FROM rust:1.93.0-bookworm

# Ensure the formatter + linter are present in this toolchain.
RUN rustup component add rustfmt clippy

WORKDIR /work
