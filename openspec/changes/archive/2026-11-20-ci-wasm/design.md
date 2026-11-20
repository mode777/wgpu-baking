# Design: ci-wasm

## Context

`rust-toolchain.toml` already installs the wasm32 target. `cargo tree --target wasm32-unknown-unknown
--no-default-features --features web -e normal` lists exactly the web dependency set (verified in P2.2: no
pollster/rayon/zstd).

## Goals / Non-Goals

**Goals:** fail fast on web-path regressions in < 5 minutes. **Non-Goals:** running wasm code (no bridge
crate or runner yet).

## Decisions

- **Dependency guard with `cargo tree` + `grep`** rather than `cargo deny`: no extra tool, and the list of
  forbidden crates (ADR-0003: pollster, rayon, zstd/zstd-sys) is explicit in the workflow.
- **Host-side web-feature unit tests** cover the `web` cfg branches (sequential `par`, KTX2 scheme 0,
  `MemoryStore`) without a wasm runner; wasm-bindgen-test arrives with the bridge.
- **Separate job** (not a step in `rust-native`) so its status names the browser build explicitly (S1).

## Risks / Trade-offs

- [Host tests of the web feature don't prove wasm32 runtime behaviour] → covered by P2.4's
  wasm-bindgen-test and P2.6's browser e2e.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none. ADRs: none (implements ADR-0009).
