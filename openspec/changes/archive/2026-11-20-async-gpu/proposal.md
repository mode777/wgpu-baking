# Proposal: async-gpu (P2.2)

## Why

The engine still blocks on the GPU: `Gpu::new` uses `pollster::block_on`, and every batch is read back
with `device.poll(wait)` + `mpsc::recv`, which deadlocks in the browser (FEASIBILITY.md B1, B2, B8). The
instance picks backends from the environment (B3), the device requests whatever limits the adapter has, and
a software adapter is used silently. Phase 2 needs a bake that awaits the GPU, opens WebGPU explicitly,
reports the adapter and refuses fallback adapters unless allowed (AGENTS.md rule 2, ADR-0003 points 2, 7, 8).
Requirements: REQ-WASM-02, REQ-WASM-04, REQ-WASM-06 (adapter info), REQ-WASM-08 (limits), REQ-WASM-10.

## What Changes

- **BREAKING (engine API):** `bake()` and `Gpu::run()` are `async`; `Gpu::open(&GpuOptions)` is public and
  async. Native callers (CLI, tests, examples) wrap with `pollster::block_on`; `Gpu::new()` stays (native).
- Readback via `map_async` + a `futures-channel` oneshot; native polls the device before awaiting, the web
  only awaits.
- Instance: `Backends::BROWSER_WEBGPU` under `web`; native keeps environment selection (`WGPU_BACKEND`).
- Adapter policy in the engine: software/fallback adapters are refused unless `GpuOptions::allow_software`;
  typed `GpuError` (no adapter, software adapter refused, no device, limit exceeded).
- `AdapterReport` (name, vendor, device type, backend, driver, software flag, granted limits) on `Gpu`.
- Explicit limits: WebGPU defaults with `maxStorageBufferBindingSize`/`maxBufferSize` raised to the
  adapter's; buffer sizes checked against them before allocation (`LimitExceeded` instead of a panic).
- CLI: `--allow-software-adapter` (or `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1`); prints the adapter report.

## Capabilities

### New Capabilities

- `gpu-access`: how the engine opens a GPU, which adapters it accepts, which limits it requests, how it
  reports them, and that it never blocks the caller while waiting for the GPU.

### Modified Capabilities

None.

## Out of scope

Progress and cancel hooks (P2.4/P2.7), device-lost handling (P2.7), the wasm-bindgen bridge (P2.4), adaptive
batch sizes (Phase 6), keeping data on the GPU (Phase 6).

## Impact

`crates/lightbaker` (`gpu/mod.rs`, `bake.rs`, `volume.rs` signatures; `futures-channel` dependency), CLI,
tests, examples, ADR-0003 (points 2, 7, 8 implemented), INTERFACES.md §9, BUILD.md, FEASIBILITY.md.
