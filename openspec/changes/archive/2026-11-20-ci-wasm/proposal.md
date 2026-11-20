# Proposal: ci-wasm (P2.3)

## Why

S1 (Phase 2 exit criterion 1) requires that `lightbaker` compiles to `wasm32-unknown-unknown` **and that
CI enforces it**. P2.1/P2.2 made the build pass locally; nothing stops a later change from adding a
native-only dependency or blocking call to the web path. ADR-0009 defines a `wasm` job. Requirements:
REQ-WASM-01, REQ-WASM-10, REQ-ENG-04.

## What Changes

- New CI job `wasm` (ubuntu-latest): `cargo build -p lightbaker --target wasm32-unknown-unknown
  --no-default-features --features web --locked`, plus the web-feature unit tests on the host
  (`cargo test -p lightbaker --no-default-features --features web --lib`) and a dependency guard that fails
  if `pollster`, `rayon` or `zstd` appear in the web dependency tree.
- The job is structured so P2.4 adds `wasm-pack build` and `wasm-bindgen-test` (Node) for
  `crates/lightbaker-web`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci-pipeline`: adds the requirement that CI builds and checks the browser (wasm32 + `web`) configuration.

## Out of scope

`wasm-pack`/`wasm-bindgen-test` (P2.4, when the bridge crate exists), the `web` job with browsers (P2.6).

## Impact

`.github/workflows/ci.yml`, BUILD.md CI table.
