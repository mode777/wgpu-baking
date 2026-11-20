# Proposal: wasm-bridge (P2.4)

## Why

The engine now compiles to wasm32 and awaits the GPU, but nothing exposes it to JavaScript. The browser PoC
(P2.5) and its e2e test (P2.6) need a wasm-bindgen API to check WebGPU support, open the baker, bake a scene
with progress, cancel, react to device loss and check results against `expect.json`
(INTERFACES.md §7–8). Requirements: REQ-WASM-02, REQ-WASM-03, REQ-WASM-06, REQ-WASM-07 (callback),
REQ-WASM-08 (typed errors), REQ-WASM-10.

## What Changes

- New crate `crates/lightbaker-web` (cdylib + rlib): `capabilities()`, `WebBaker.create(allowFallbackAdapter)`,
  `bake(scene, configJson, onProgress)` → `Promise<BakeOutput>`, `cancel()`, `evaluate(expectJson)`,
  `adapter()`, `clearCache()`, `setDeviceLostCallback(cb)`. Errors reject with `{ kind, message }`
  (`BakerError` kinds plus `cancelled`).
- Config overrides: a partial `BakeConfig` JSON applied to the scene's settings (lightmap density,
  padding, downscale, chart angle, sampling, GI, AO, `incremental.reuseTiles`); unsupported fields are
  reported as warnings.
- Engine: `Options` gains `progress` and `cancel` hooks (per GPU batch and per stage); `Cancelled` error;
  `Gpu::probe()` (adapter report without a device) for capability checks; `Gpu` records uncaptured GPU
  errors and device loss instead of panicking, and fails the running batch with them;
  `Gpu::set_device_lost_callback`.
- Engine fix: the WebGPU-only instance and the "no blocking poll" path key on `target_arch = "wasm32"`
  instead of the `web` feature, because Cargo unifies features in workspace builds.
- CI `wasm` job: `wasm-pack build` and `wasm-pack test --node` for the bridge; clippy `-D warnings` for
  the new crate.

## Capabilities

### New Capabilities

- `wasm-bridge`: the JavaScript API of the baker (capabilities, create, bake, cancel, evaluate, errors).
- `bake-control`: progress reporting and cancellation of a running bake.

### Modified Capabilities

- `gpu-access`: the browser instance/poll rule applies to wasm32 builds regardless of features; GPU
  validation errors and device loss become bake errors instead of panics.

## Out of scope

The worker, page and Three.js view (P2.5), browser e2e (P2.6), worker recreation and real device-loss
tests (P2.7), `unwrap` (Phase 3), scene v2 / multi-page results (Phase 3), IndexedDB persistence.

## Impact

New crate, workspace members, `crates/lightbaker` (`progress.rs`, `gpu`, `bake.rs`, `volume.rs` signatures),
CLI/tests/examples (`..Default::default()` on `Options`), CI, INTERFACES.md §7–9, BUILD.md, notices.
