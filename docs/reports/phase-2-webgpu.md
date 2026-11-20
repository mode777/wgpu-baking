# Phase 2 report — WASM / WebGPU proof of concept

- **Roadmap:** Phase 2 (P2.1–P2.8), milestone **M2 Baker in the browser** (S1, S2).
- **Date:** 2026-11-20
- **Raw data:** [phase-2/](phase-2/) (JSON from `app/scripts/hw-check.mjs` and the native `baseline` example,
  screenshots, native-vs-browser parity).

## Exit criteria (hard gate for Phase 3)

| # | Criterion | Result | Evidence |
|---|---|---|---|
| 1 | S1: `lightbaker` compiles to `wasm32-unknown-unknown`; CI enforces it | **met** | CI `wasm` job: wasm32 build with `--features web`, guard against `pollster`/`rayon`/`zstd` on the web path, bridge clippy, `wasm-pack build`, `wasm-pack test --node` (P2.3–P2.4). |
| 2 | S2: the upstream test scene bakes in Chrome/Edge on a **hardware** WebGPU adapter (recorded) and on a software adapter in CI; metrics match native; adapters recorded | **met** (Chrome; Edge blocked by machine policy, see below) | Chrome 157 on Intel `xe-3lpg` (D3D12 via Dawn): 19/19 checks, every probe value equal to native at 3 decimals ([parity](phase-2/parity-chrome-intel-vs-native.json)). CI `web` job: Chromium's SwiftShader adapter (labelled), 19/19 checks with the `ci` profile; native `rust-gpu` on lavapipe (ADR-0009 finding). |
| 3 | Cancel and device-loss behaviours tested; native build and tests still green | **met** | Rust: cancel within one batch, cancelled bake leaves the store empty, `device_lost_mid_bake`. Vitest: protocol + `BakerService` recovery. Browser: cancel → rebake in `bake-poc.spec.ts` (CI) and on the Intel GPU; **real `device.destroy()`** on the Intel GPU: loss shown after 315 ms, Bake disabled, "Recreate baker" → new device → bake passes 19/19 ([chrome-intel-full.json](phase-2/chrome-intel-full.json) `deviceLoss`). All CI jobs green. |

**Edge:** Microsoft Edge on this machine refuses automation ("Headless mode is disallowed by the
system admin", "DevTools remote debugging is disallowed"): enterprise policy, not a WebGPU issue. Edge
uses the same Dawn/D3D12 stack as Chrome; it is part of the P6.8 browser × GPU matrix on an
unmanaged machine.

## Machines and adapters

| Run | Browser / runtime | Adapter (as reported) | Software |
|---|---|---|---|
| Native | `baseline` example, wgpu 30 | Intel(R) Graphics, Vulkan and DX12; storage binding 1023/2047 MiB | no |
| Browser, hardware | Chrome 157.0.8103.80 (headless, `--enable-unsafe-webgpu`) | `intel xe-3lpg` (GPUAdapterInfo), backend BrowserWebGpu; storage binding 2047 MiB, buffer 2048 MiB | no |
| Browser, software (local) | Chrome 157, `--use-webgpu-adapter=swiftshader` | `google swiftshader`, `isFallbackAdapter` | **yes** (refused without `allowFallbackAdapter=1`, labelled with it) |
| Browser, CI | Playwright Chromium, headed under xvfb, ubuntu-latest | `google swiftshader` (Chromium exposes no other adapter there, ADR-0009) | **yes** |
| Native, CI | `rust-gpu` job | `llvmpipe (LLVM 21.1.8, 256 bits)` (lavapipe) | **yes** |

Host: Windows 11 Enterprise, Intel Core Ultra 5 335, 31.5 GiB RAM, Intel Graphics driver 32.0.101.8831.

## Native vs browser (Intel iGPU, same scenes and settings)

Native: median of 5 bakes after a warm-up (`baseline`). Browser: median of 5 (`full`) or 3 (`dense`) warm
bakes with tile reuse off, after a first bake (`hw-check.mjs --bench`). Browser phases end at the last
batch of each phase (progress events), so "direct" includes rasterisation and "bounce 1" the CPU per-tile
direct stage, as in the native breakdown.

### `upstream-basic` (538 triangles, atlas 1024×180, 256 rays, 2 bounces)

| Stage (ms) | Native Vulkan | Native DX12 | Chrome (D3D12), warm | Chrome, first bake |
|---|---:|---:|---:|---:|
| layout, hashes | 2.3 | 2.4 | 1 | 6 |
| rasterise + direct (GPU) | 12.1 | — | 18 | 350 |
| CPU per-tile direct + bounce 1 | 20.3 | — | 109 | 150 |
| bounce 2 | 8.1 | — | 22 | 25 |
| volume | 6.2 | — | 14 | 18 |
| **total** | **53.9** | **58.1** | **176** | **575** |

### `upstream-basic-dense` (atlas 2048×924, 1.1 M texels, 512 rays, 3 bounces)

| Stage (ms) | Native Vulkan | Chrome (D3D12), warm | Ratio |
|---|---:|---:|---:|
| layout, hashes | 2.7 | 1 | — |
| rasterise + direct (GPU) | 196.8 | 474 | 2.4× |
| CPU per-tile direct + bounce 1 | 408.5 | 1720 | 4.2× |
| bounce 2 | 154.8 | 373 | 2.4× |
| bounce 3 | 160.0 | 367 | 2.3× |
| volume | 8.2 | 94 | — |
| atlas assembly | 72.2 | (in volume/total) | — |
| **total** | **1015** | **3150** | **3.1×** |

Native DX12 dense: 979 ms. SwiftShader in Chrome, `ci` profile, warm: 610 ms (first 1.31 s); native
lavapipe (full profile, P1.5): 599 ms.

### Transfers, limits, memory

| | upstream-basic | dense |
|---|---:|---:|
| GPU batches = readbacks | 7 | 72 |
| read back | 1.85 MiB | 30.01 MiB |
| uploaded | 15.29 MiB | 234.99 MiB |
| light atlas (16 B/texel) | 2.8 MiB | 28.9 MiB |
| wasm memory after bake (browser) | 36.6 MiB | 469 MiB |
| native peak working set (process, P1.5) | — | ≈ 800 MiB (incl. driver) |

Identical transfer counts native and browser (same code path, P2.2 only made it async).

- **Limits:** none hit. Requested: WebGPU defaults with `maxStorageBufferBindingSize`/`maxBufferSize` raised
  to the adapter's (Chrome/Intel: 2047/2048 MiB; SwiftShader: 1024/1024 MiB). The largest binding (dense
  `texels`, ≈ 68 MiB) is below even the 128 MiB WebGPU default.
- **WGSL:** upstream's `trace.wgsl` compiled unmodified with Tint (Chrome, D3D12 and SwiftShader/Vulkan),
  no uniformity-analysis errors or warnings. The first bake's "direct" phase (≈ 330 ms extra on the
  Intel iGPU) is pipeline compilation: Chrome compiles the three compute pipelines lazily.
- **wasm memory** only grows: 469 MiB for the dense scene (the `texels` array is 68 MiB, `light` 29 MiB,
  plus per-tile arrays, copies for upload and the result arrays). This is the main risk for larger scenes
  under the wasm32 4 GiB ceiling (FEASIBILITY B9).

## Observations

1. **Results match native exactly** on the probes (Vulkan native vs D3D12 in Chrome) and the bake is
   deterministic on each side; Tint and naga agree on the shader.
2. **The browser is ≈ 3× slower than native on equal work**, for two reasons:
   - the CPU per-tile direct stage (sun field, AO, filters) runs single-threaded in wasm instead of on
     rayon threads: 4.2× slower, 55 % of the dense browser bake;
   - GPU passes are ≈ 2.4× slower: each `run` re-uploads the whole `light` atlas and texel array through
     `queue.writeBuffer` (JS copies) and awaits a readback per batch, and Dawn adds validation.
   Both are Phase 6 work (REQ-WASM-05: keep `light` on the GPU, fills/filters as compute passes).
3. **First bake cost** ≈ 0.3–0.4 s of pipeline compilation: worth pre-warming at worker start (Phase 5).
4. **Software adapters** are correct (19/19) and ≈ 3–4× slower than the iGPU for this scene — fine for CI.
5. **Robustness found by the browser runs**: the page must work without WebGL (the CI no-WebGPU browser has
   none) — fixed in P2.6; Chrome need not flag lavapipe-like adapters as fallback, so the bridge checks
   `GPUAdapterInfo` vendor/architecture too (P2.5).

## Performance targets for Phases 3–6 (Q4)

Reference: Chrome on the Intel Xe-3LPG iGPU above ("reference iGPU"); `dense` = `upstream-basic-dense`
settings. Times are warm bakes with tile reuse off unless stated. Phase reports must re-measure these with
`hw-check.mjs --bench` and the native `baseline` example.

| Target | Now (P2) | Phase 3 | Phase 4 | Phase 5 | Phase 6 |
|---|---:|---:|---:|---:|---:|
| `upstream-basic`, browser | 176 ms | ≤ 250 ms | ≤ 300 ms | ≤ 300 ms | ≤ 120 ms |
| `dense`, browser | 3.15 s | ≤ 4 s | ≤ 5 s (with 8 point lights + emissive) | ≤ 5 s | **≤ 1.5 s** |
| browser / native ratio (`dense`) | 3.1× | ≤ 3.5× | ≤ 3.5× | ≤ 3.5× | **≤ 1.5×** |
| uploads per `dense` bake | 235 MiB | ≤ 250 MiB | ≤ 300 MiB | — | ≤ 2 × atlas (≈ 60 MiB) |
| readbacks per bake | every batch | every batch | every batch | — | final pages only (REQ-WASM-05) |
| wasm memory, `dense` | 469 MiB | ≤ 512 MiB | ≤ 512 MiB | ≤ 512 MiB | ≤ 256 MiB |
| rebake after moving one object (tile reuse), showcase vignette | — | — | — | ≤ 2 s | ≤ 1 s |
| UI frame time during a bake (worker) | — | — | — | < 33 ms | < 20 ms |
| Balanced preset, showcase plaza (≤ 100 k triangles, ≤ 2 pages of 2048²) | — | — | — | ≤ 60 s | **≤ 30 s** |
| Preview preset, same scene | — | — | — | ≤ 10 s | ≤ 5 s |

Supported scene size (Phase 6 commitment, reference iGPU, Balanced): ≤ 250 k triangles and ≤ 4 pages of
2048² without `limit-exceeded` or `out-of-memory`; larger scenes fail with a clear `limit-exceeded` or
`out-of-memory` error, never a crash (REQ-WASM-08).

## Changes made in Phase 2

- P2.1 `native`/`web` features, `par` and `web-time` shims, `TileStore` (`FsStore`/`MemoryStore`), web KTX2
  without zstd.
- P2.2 async `Gpu::open`/`run`/`bake`, awaited readback, adapter policy and report, explicit limits.
- P2.3 CI `wasm` job.
- P2.4 `crates/lightbaker-web` (capabilities, `WebBaker`, typed errors), engine progress/cancel hooks, GPU error
  and device-loss capture, platform selection by `target_arch`.
- P2.5 `app/` PoC page, worker protocol, Three.js lightmap view, `js` `readScene`, `hw-check.mjs`.
- P2.6 Playwright e2e (`bake-poc`, `no-webgpu`), CI `web` job, page works without WebGL.
- P2.7 `BakerService` recovery from device loss, debug `device.destroy()`, cancelled bakes leave the store empty.
- P2.8 `dense` profile, `--bench`, `wasmMemoryBytes`, this report.

Upstream files changed (additive or `cfg`-gated): `bake.rs` (async, hooks, `TileStore`), `gpu/mod.rs`
(async, adapter policy, limits, error capture, uploads via `queue.write_buffer`), `volume.rs` (async),
`cache.rs` (trait), `encode.rs` (scheme by feature), `deps.rs`/`bake.rs` imports, `lib.rs` (modules),
CLI (`block_on`, flags), `tests/bake.rs` (shared helpers). Native `.lbake` bytes are unchanged
(`B80AD1…` for `upstream-basic` before and after Phase 2).

## Risks re-assessed (FEASIBILITY §9)

| Risk | Status |
|---|---|
| Browser WGSL compiler rejects/miscompiles the shader | **retired**: Tint compiles it; results match native exactly |
| Readback-heavy pipeline slow in browsers | **confirmed** (≈ 2.4× on GPU passes); Phase 6 plan unchanged |
| Single-threaded CPU stages slow | **confirmed** (4.2×, the largest share); Phase 6: move fills/filters to compute |
| wasm memory for large scenes | **new emphasis**: 469 MiB for 1.1 M texels; budget checks (B9) and GPU-resident data in Phase 6 |
| Software adapters in CI hide real-GPU bugs | mitigated by this report's hardware runs; CI browser uses SwiftShader, not lavapipe (ADR-0009) |
