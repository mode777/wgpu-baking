# Spec Delta

## ADDED Requirements

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
