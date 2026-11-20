# Proposal: cancel-and-device-loss (P2.7)

## Why

Phase 2 exit criterion 3 requires cancel and device-loss behaviour to be tested. The engine already stops
within one batch and turns device loss into an error (P2.4), but the page cannot recover from a lost device
without a reload, nothing proves a cancelled bake leaves the tile store untouched, and the real
`device.destroy()` path has not been exercised in a browser (ADR-0002, TESTING.md). Requirements:
REQ-WASM-06 (cancel), REQ-WASM-07 (device loss and recovery), REQ-APP-06.

## What Changes

- `app/src/bake/service.ts`: `BakerService` owns the worker through a factory; on `device-lost` it marks
  the baker lost and `recreate()` terminates the worker, starts a new one and re-initialises — no reload.
- Page: on device loss, a "Recreate baker" button; with `?debug=1`, a "Lose device" button that sends a
  `debug-lose-device` request (the worker calls the bridge's `loseDeviceForTesting()`, i.e. `device.destroy()`).
- Worker core: handles `debug-lose-device`; device loss is reported once per baker.
- Rust test: a bake cancelled during a bounce writes nothing to its tile store and the next bake is unaffected.
- Vitest: device loss (mocked `GPUDevice.lost` through the fake bridge) during and outside a bake → error
  kinds, service state `lost`, `recreate()` gives a working baker.
- `hw-check.mjs --lose-device`: real `device.destroy()` on the hardware GPU → message → recreate → bake passes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `browser-baking`: recovery from device loss without reloading the page.
- `bake-control`: a cancelled bake leaves the tile store unchanged.

## Out of scope

Device-loss recovery under load and adaptive batches (P6.5), persisting baked tiles across workers.

## Impact

`app/src/bake/` (service, core, protocol), `app/src/main.ts`, `app/index.html`, `app/scripts/hw-check.mjs`,
`crates/lightbaker/tests/scenes.rs`, TESTING.md, INTERFACES.md §7, ADR-0002 status.
