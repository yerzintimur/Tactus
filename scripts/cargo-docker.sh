#!/usr/bin/env bash
# Run `cargo` inside the pinned Tactus core Docker image — reproducible builds.
#
# Scope: the Rust core + its tests/lints + tooling (all Linux-able work). The iOS
# leg must run on host macOS + Xcode — do NOT route iOS builds through here.
#
# Examples:
#   scripts/cargo-docker.sh test
#   scripts/cargo-docker.sh build --release
#   scripts/cargo-docker.sh clippy --all-targets -- -D warnings
#   scripts/cargo-docker.sh fmt --check
set -euo pipefail

IMAGE="tactus-core:1.93.0"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Build the image on first use (or after the Dockerfile changes and it's removed).
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo ">> building $IMAGE (one-time) ..." >&2
  docker build -t "$IMAGE" -f "$ROOT/docker/Dockerfile" "$ROOT"
fi

# Caches + build dir live under .docker/ (git-ignored), separate from any host
# `target/`, so host-native and container artifacts never collide.
mkdir -p "$ROOT/.docker/cargo" "$ROOT/.docker/target"

exec docker run --rm \
  --user "$(id -u):$(id -g)" \
  -e CARGO_HOME=/work/.docker/cargo \
  -e CARGO_TARGET_DIR=/work/.docker/target \
  -e HOME=/work/.docker \
  -v "$ROOT:/work" -w /work \
  "$IMAGE" cargo "$@"
