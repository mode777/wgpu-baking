# Proposal: ci-native (P1.3)

## Why

Phase 1 exit criterion 1 requires `cargo test -p lightbaker` to pass with a GPU **in CI on lavapipe**, not
skipped. Today the upstream GPU test silently returns when no adapter is found, so CI could be green
without testing anything; and nothing prevents a software adapter from being used unannounced (AGENTS.md
rule 2). ADR-0009 fixes the CI design. Requirements: REQ-ENG-03, REQ-ENG-04, REQ-WASM-10.

## What Changes

- `.github/workflows/ci.yml` with two jobs:
  - `rust-native` on `ubuntu-latest` and `windows-latest`: build the workspace, run unit tests, clippy
    with `-D warnings` for crates listed as new (none yet; list grows from Phase 2).
  - `rust-gpu` on `ubuntu-latest` with `mesa-vulkan-drivers` (lavapipe): `cargo test -p lightbaker` with
    `LIGHTBAKER_REQUIRE_GPU=1` and `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1`.
- GPU test helper (`crates/lightbaker/tests/bake.rs`): honour `LIGHTBAKER_REQUIRE_GPU` (no adapter →
  fail) and `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER` (software adapter → skip unless allowed, fail when
  required and not allowed), and print the adapter used so software results are labelled.

## Capabilities

### New Capabilities

- `ci-pipeline`: automated checks on every push and pull request, and the GPU-test adapter policy.

### Modified Capabilities

None.

## Out of scope

- `wasm` job (P2.3), `web` job (P2.6), optional `gpu-perf` job.
- Adapter reporting inside the engine (`Gpu` adapter report is P2.2); here detection lives in the test helper.
- Fixing upstream clippy warnings (upstream files are not linted, ADR-0001).

## Impact

New `.github/workflows/ci.yml`; small additive change in the upstream test file; BUILD.md, TESTING.md.
