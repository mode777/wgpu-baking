# Design: async-gpu

## Context

`Gpu::open` (private, async) builds the instance from the environment, picks a `HighPerformance` adapter and
requests `adapter.limits()`. `Gpu::run` loops batches; each batch submits, maps, `poll(wait_indefinitely)`
and `mpsc::recv`s. `bake()` calls `run` three times (direct, each bounce) and `bake_volume` once, always
outside rayon closures.

## Goals / Non-Goals

**Goals:** one async code path for both platforms; same results natively; clear typed errors.
**Non-Goals:** concurrency between batches (each still awaits its readback before the next).

## Decisions

- **`async fn run` / `async fn bake` / `async fn bake_volume`.** Only the call chain that reaches `run`
  becomes async; the CPU stages are untouched. The future is not required to be `Send` (wgpu types on wasm
  are not).
- **Readback:** `futures_channel::oneshot` + `map_async`. Under `native` the device is polled with
  `PollType::wait_indefinitely()` before awaiting (wgpu native only runs callbacks when polled); under
  `web` the browser drives the promise while we await. Alternative: `wgpu::util::DownloadBuffer` — same
  mechanism, less control over labels/counters.
- **Instance:** `cfg(feature = "web")` → `InstanceDescriptor { backends: BROWSER_WEBGPU, .. }`; otherwise
  `new_without_display_handle_from_env()` (keeps `WGPU_BACKEND` for local testing, as used in P1.3/P1.5).
- **Adapter policy in the engine** (moved from the test helper): `GpuOptions { allow_software }`; refusal is a
  pure function `admit(software, allow)` so it is unit-testable without a software adapter. The test helper
  and CLI pass the opt-in; the helper keeps the skip/fail logic for "no usable GPU".
- **`GpuError`** (`thiserror`-free enum implementing `std::error::Error`), carried inside `anyhow::Error`
  so callers can `downcast_ref::<GpuError>()` to map to `BakerError` kinds (P2.4).
- **Limits:** `required_limits(adapter) = Limits { max_storage_buffer_binding_size: adapter's,
  max_buffer_size: adapter's, ..Limits::defaults() }`. Defaults are the WebGPU baseline, so every conformant
  adapter grants them; raising only the two buffer limits lets large atlases bind where the adapter allows.
  `Gpu::storage` and the per-batch buffers check sizes against the granted limits first.
- **`Gpu::new()`** (native) = `block_on(open(&GpuOptions::default()))`, i.e. software refused by default.

## Risks / Trade-offs

- [Some native adapters (downlevel GL) don't grant WebGPU defaults] → not a target (wgpu native backends
  used are Vulkan/DX12/Metal); request fails with `NoDevice` and a clear message.
- [Changing `Gpu::new()` to refuse software adapters] → intended (rule 2); CI and tests opt in via env.

GPU-to-CPU readbacks: none new (same per-batch readbacks, now awaited). Blocking calls: `pollster::block_on`
remains only in native callers (`Gpu::new`, CLI, tests, examples); `device.poll(wait)` only under `native`.
Native-only dependencies: none new (`futures-channel` is pure Rust and builds for wasm32).
ADRs: ADR-0003 updated (points 2, 7, 8 implemented).
