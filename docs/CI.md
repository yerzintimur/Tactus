# Continuous Integration (CI)

Every push to `main` (and every pull request) is automatically built and tested
on GitHub's servers. This page explains how — written for a first encounter
with GitHub Actions, and ending with a primer on how mobile *releases* will
work later (CI does not release anything today).

The pipeline itself is one file: [.github/workflows/ci.yml](../.github/workflows/ci.yml).

---

## GitHub Actions in five concepts

| Concept | What it is | In this repo |
|---|---|---|
| **Workflow** | A YAML file in `.github/workflows/` describing what to run and when. | `ci.yml`, named "CI". |
| **Trigger** (`on:`) | The event that starts a workflow. | Push to `main`, any pull request, or the manual **Run workflow** button (`workflow_dispatch`). |
| **Job** | A group of steps on one fresh virtual machine. Jobs run in parallel. | `core` and `apple`. |
| **Runner** | The VM a job runs on. GitHub hosts Linux, Windows, macOS. | `ubuntu-latest` for the core, `macos-latest` for the Apple leg (the Apple toolchain only exists on macOS). |
| **Step** | One shell command or a reusable **action** (a published building block, versioned like `actions/checkout@v5`). | Checkout, caches, then the same `just` recipes you run locally. |

Where to look: the **Actions** tab of the repo lists runs; click a run → a job
→ a step to read its log. A red run can be re-run from the same page
(**Re-run failed jobs**) — useful for one-off flakes.

**Cost:** for a **public** repo (ours), GitHub-hosted runners are **free,
including macOS**. (If the repo ever goes private: the free plan gives
2 000 runner-minutes/month, and macOS minutes are billed at **×10** — the
Apple job would eat the quota fast.)

**Trust note:** third-party actions run with access to the repo, so we pin to
well-known ones only (`actions/*` by GitHub, `taiki-e/*` — a de-facto standard
in the Rust ecosystem for installing tools with caching).

---

## Our pipeline

One design rule: **CI runs the same `just` recipes as local dev.** The justfile
stays the single source of truth; nothing is duplicated in YAML. Any red step
is reproducible on your machine with the exact command shown in the log.

| Job | Runner | What it runs | Local equivalent |
|---|---|---|---|
| **Rust core** | `ubuntu-latest` | fmt check, clippy `-D warnings` (incl. the `simffi` feature), all tests, then the virtual-device e2e suite — all **inside the same pinned Docker image** as local dev (`scripts/cargo-docker.sh` builds it on first use). | `just test-core` + `just test-e2e` |
| **Apple app** | `macos-latest` | Build the XCFramework + Swift bindings, generate the Xcode project, run unit + UI tests (the full simulated-module pipeline **and the accessibility-audit gate**) on an iOS Simulator. | `just build-ios` + `just ios-gen` + `just ios-test` |

Details worth knowing:

- **Reproducibility.** The core job doesn't install Rust on the runner at all —
  it runs the identical `rust:1.93.0` Docker image you use locally, so "works
  on my machine" and "works in CI" are the same statement. The Apple job pins
  Rust via `rust-toolchain.toml` (rustup auto-installs it).
- **Simulator pick.** Runner images ship different simulator sets per Xcode
  version, so the workflow picks the newest available iPhone at runtime instead
  of hardcoding a model.
- **No secrets, no signing.** Simulator builds don't need code signing, and the
  workflow needs no credentials of any kind. The moment a workflow needs a
  secret (e.g. a future release), it goes into **Settings → Secrets and
  variables → Actions**, never into the repo.
- **Caching.** `actions/cache` snapshots the Cargo registry and build dirs,
  keyed on the hash of `Cargo.lock` + `rust-toolchain.toml`; `restore-keys`
  lets a slightly-stale cache still warm up the build after a dependency
  change. First run is slow (~15–20 min); warm runs are much faster.
- **Auto-cancel.** A new push to the same branch cancels the previous,
  still-running CI (`concurrency`) — no queue of obsolete runs.
- The README badge shows the latest state of `main` at a glance.

### When CI goes red

1. Open the run → the failing job → the failing step; the log ends with the
   actual error.
2. Reproduce locally with the same recipe (`just test-core`, `just ios-test`, …).
3. Simulator/infra flake (rare, but real on shared macOS runners)? **Re-run
   failed jobs** once before digging.

---

## What CI deliberately does *not* do (yet)

- **No releases.** Nothing is signed, packaged, or uploaded anywhere.
- **No Android job.** Added together with the Android scaffold (M5): the same
  Docker image gains cargo-ndk + the Android NDK, plus a Gradle/Compose build
  and its accessibility gate.
- **No hardware tests.** Anything needing a real V31 stays a manual, on-host
  session ([HARDWARE_TESTING.md](HARDWARE_TESTING.md)); CI covers the same
  paths through the simulated module instead.

---

## Primer: how mobile releases will work (for later)

Nothing below exists yet — this is the map for when we ship.

### iOS

1. **Apple Developer Program** ($99/year) — required for TestFlight and the
   App Store. (The free personal team you use for on-device dev runs can't
   distribute, and its installs expire after 7 days.)
2. **Signing, two flavours.** *Development* signing is what already puts dev
   builds on your iPhone. *Distribution* signing (a separate certificate +
   provisioning profile, issued in App Store Connect) is what store uploads
   require. Xcode's "Automatically manage signing" handles both once the
   program membership exists.
3. **The pipeline to users:**
   `xcodebuild archive` (Release, distribution-signed) → upload to
   **App Store Connect** → **TestFlight** (instant for your own devices;
   external beta testers pass a light review; up to 10 000 testers) →
   **App Review** → the App Store.
4. **Where CI hooks in later:** a separate `release.yml` workflow, triggered by
   a version tag (e.g. `v0.2.0`), building with `just build-ios-release`
   (bindings *without* the simulated module — the recipe asserts its symbols
   are absent), archiving, and uploading via the **App Store Connect API key**
   stored in GitHub secrets. Tools like *fastlane* automate the
   certificate/upload choreography; we'll pick tooling when we get there.

### Android

1. **Google Play Console** — one-time $25.
2. Build a signed **AAB** (Android App Bundle); the upload key lives in GitHub
   secrets, and **Play App Signing** holds the real signing key server-side.
3. Release tracks: internal → closed → open testing → production, all from the
   Play Console; CI uploads via the Play Developer API.

### The one rule that never changes

Store credentials, signing keys, and API tokens live **only** in GitHub
secrets (or a local keychain) — never in the repo, never in a workflow file,
never in a log.
