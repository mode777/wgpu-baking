# Proposal: xatlas-native (P3.5)

## Why

The native engine (CLI, tests, CI) needs the same automatic lightmap UVs as the browser (ADR-0006): xatlas from
the same pinned source, called through a contract the bake can use for object tiles (P3.7), with the UV validity
suite TESTING.md requires. Requirements: REQ-UV-01 … 05, REQ-WASM-10 (native and wasm builds).

## What Changes

- Engine contract `lightbaker::unwrap`: `Unwrapper` trait, `MeshInput`, `UnwrapOptions`, `UnwrappedTile` /
  `UnwrappedMesh`, and the UV validity checks (`check_uvs`) — pure Rust, built for every target. The contract
  lives in the engine so `lightbaker` keeps depending on none of our crates (AGENTS.md); implementations depend on it.
- New crate `crates/lightbaker-uv`: vendored `jpcy/xatlas` at `f700c77` (MIT, licence kept) compiled with `cc`
  (`XATLAS_C_API=1`, `XA_MULTITHREADED=0`, like `xatlas-wasm`), hand-written `extern "C"` bindings (no bindgen),
  `XatlasUnwrapper` implementing `Unwrapper`. On wasm32 the crate builds without the FFI (the web uses
  `xatlas-wasm` in the worker, P3.6).
- Tests: the UV validity suite natively on the test meshes (quad, 3-material box, `upstream-basic` meshes, UV
  sphere, torus knot), tile sizing, determinism.
- CI: `lightbaker-uv` linted (clippy, fmt) and built for wasm32.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `lightmap-unwrap`: the contract and validity guarantees now also hold natively; adds determinism.

## Out of scope

Using the unwrap in the bake (P3.6/P3.7), web adapter in Rust (P3.6), caching by geometry hash (P3.7).

## Impact

New crate and vendored C++ (~330 KB source), `crates/lightbaker/src/unwrap.rs`, workspace, CI, AGENTS.md
(layout note), ARCHITECTURE §4, ADR-0006 (contract location), BUILD.md (C++ compiler), THIRD_PARTY_NOTICES.
