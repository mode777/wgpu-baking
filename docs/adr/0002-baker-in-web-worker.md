# ADR-0002: Bake in a dedicated Web Worker with its own WebGPU device

- **Status:** accepted — confirmed by the Phase 2 PoC (P2.5–P2.7): the baker runs in a module worker,
  cancels within one GPU batch, and recovers from device loss by recreating the worker
  (`app/src/bake/service.ts`)
- **Date:** 2026-11-20
- **Phase:** 2
- **Requirements:** REQ-WASM-02, REQ-WASM-06, REQ-WASM-07, REQ-APP-06

## Context

A bake runs for seconds to minutes: CPU stages (layout, xatlas, BVH build, filters) are
synchronous Rust; GPU stages are many batched dispatches. On the main thread this would freeze
the editor. WebGPU is available in dedicated workers (`navigator.gpu` in `DedicatedWorkerGlobalScope`).
Three.js renders on the main thread with its own device/context.

## Decision

- The WASM baker runs in a **dedicated module Web Worker** (`app/src/bake/baker.worker.ts`), with
  its own `GPUAdapter`/`GPUDevice` created by wgpu.
- Communication uses the protocol in [INTERFACES.md §7](../INTERFACES.md#7-worker-protocol);
  results are transferred (`Transferable`) as typed arrays and uploaded by Three.js as textures.
- Cancellation: the worker processes `cancel` messages whenever the bake awaits (every GPU batch);
  a `cancel` flag in Rust is checked after each await. CPU-only stages that run long (xatlas on big
  meshes, BVH) are split so progress/cancel are observed at least every ~250 ms where practical.
- Device loss: wgpu's device-lost callback rejects the running bake with `device-lost`; the UI
  offers "recreate baker" (new worker), no page reload.

## Alternatives considered

| Option | Pros | Cons |
|---|---|---|
| Share Three.js' `GPUDevice` on the main thread | No copy of results; lightmap could stay a GPU texture | Blocks the UI during CPU stages; couples the engine to the renderer; WebGL fallback renderer has no device to share |
| `OffscreenCanvas` render in the same worker | One device | Moves the whole editor rendering off-main-thread; large complexity for React/Three integration |
| SharedArrayBuffer + wasm threads | Parallel CPU stages | Needs COOP/COEP headers and nightly-ish wasm threading; deferred (possible Phase 6 optimisation) |

## Consequences

- One extra GPU→CPU→GPU copy of the final lightmap (acceptable: tens of MB at most, once per bake).
- Progressive previews are partial copies per tile/batch (rate-limited).

## Verification

Phase 2 spike: bake the upstream test scene in a worker; UI remains responsive (frame time
recorded); cancel works within one batch.
