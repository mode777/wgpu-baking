# ci-pipeline Specification

## Purpose
Defines the automated checks that run on every push and pull request and how GPU tests treat missing or
software GPU adapters, so CI cannot pass without exercising the GPU code.

## Requirements

### Requirement: Native CI checks on Linux and Windows
CI SHALL build the Rust workspace and run its tests on Ubuntu and Windows for every push to `main` and
every pull request, and SHALL fail the run when the build or any test fails.

#### Scenario: Failing unit test fails CI
- **WHEN** a commit makes a Rust test fail
- **THEN** the `rust-native` job for that platform fails

### Requirement: GPU tests run on a software adapter in CI
CI SHALL run the `lightbaker` GPU integration tests on Linux with a Mesa lavapipe software adapter,
with a missing adapter treated as a failure and the software adapter explicitly allowed.

#### Scenario: GPU job exercises the GPU
- **WHEN** the `rust-gpu` job runs
- **THEN** the GPU integration test runs to completion on lavapipe and its log names the adapter

### Requirement: Missing adapter fails only when required
GPU tests SHALL skip with a message when no adapter is available, unless `LIGHTBAKER_REQUIRE_GPU=1`, in
which case they SHALL fail.

#### Scenario: Local machine without GPU
- **WHEN** no adapter is found and `LIGHTBAKER_REQUIRE_GPU` is unset
- **THEN** the test passes as skipped and prints "no GPU, skipped"

#### Scenario: CI without GPU
- **WHEN** no adapter is found and `LIGHTBAKER_REQUIRE_GPU=1`
- **THEN** the test fails

### Requirement: Software adapters need explicit opt-in
GPU tests SHALL treat an adapter identified as software (CPU device type, or a name containing llvmpipe,
lavapipe, SwiftShader, "Microsoft Basic Render Driver" or WARP) as unavailable unless
`LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1`, and SHALL label results from an allowed software adapter.

#### Scenario: Software adapter not allowed
- **WHEN** only a software adapter is available and the opt-in is unset
- **THEN** the test behaves as if no adapter were found (skip, or fail when required)

#### Scenario: Software adapter allowed
- **WHEN** only a software adapter is available and `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1`
- **THEN** the test runs and its output marks the adapter as software

### Requirement: CI builds the browser configuration
CI SHALL build `lightbaker` for `wasm32-unknown-unknown` with `--no-default-features --features web` on every
push and pull request, run the engine's unit tests with the `web` feature, and fail if a blocking executor,
rayon or zstd is in the web dependency tree.

#### Scenario: Native-only dependency added to the web path
- **WHEN** a change makes `pollster`, `rayon` or `zstd` a dependency of the `web` configuration
- **THEN** the `wasm` job fails

#### Scenario: Web build breaks
- **WHEN** a change makes the wasm32 build of `lightbaker` fail
- **THEN** the `wasm` job fails
