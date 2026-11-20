# ADR-0003: Platform abstraction for native and WASM

- **Status:** accepted — points 1, 3–6 implemented in P2.1 (`platform-shims`); 2, 7, 8 in P2.2 (`async-gpu`)
- **Date:** 2026-11-20
- **Phase:** 2
- **Requirements:** REQ-WASM-01 … 03, REQ-WASM-10

## Context

The inspection ([FEASIBILITY.md §3](../FEASIBILITY.md#3-wasm-blockers-exact-locations)) found these
native-only constructs in `crates/lightbaker`:

- `pollster::block_on` in `Gpu::new` and `std::sync::mpsc` + `device.poll(PollType::wait_indefinitely())`
  in `Gpu::run` (blocks; deadlocks on the web where `poll` does not wait).
- `std::time::Instant` in `bake.rs`, `volume.rs` (panics on `wasm32-unknown-unknown`).
- `rayon` (`par_iter`, `par_iter_mut`) in `bake.rs`, `deps.rs`.
- `std::fs` in `cache.rs`; `zstd` (C library via `zstd-sys`) in `cache.rs` and `encode.rs`.
- `InstanceDescriptor::new_without_display_handle_from_env()` (environment-driven backend selection).

## Decision

1. Cargo features on `lightbaker`: `native` (default: rayon, pollster, zstd, fs cache) and `web`
   (wgpu `webgpu` backend only, `web-time`, no rayon, no zstd). WebGL is **not** enabled for the
   baker (no compute shaders; would be a silent downgrade). The workspace `wgpu` dependency has
   `default-features = false`; `native` enables `wgpu/default`, `web` enables `wgpu/webgpu`.
2. `bake()` and `Gpu::run()` become `async`. Readback: `map_async` with a `futures` oneshot
   channel; native polls with `device.poll(wait)` before awaiting, the web awaits the browser.
   `Gpu::new()` (blocking) stays for the CLI under `native`.
3. Time: a `time` module re-exporting `web_time::Instant` (works on both).
4. Parallelism: a small `par` module: `par_iter()`/`par_iter_mut()` map to rayon under `native` and
   to sequential iterators otherwise (same results; determinism preserved).
5. Cache: `trait TileStore` with `FsStore` (native; upstream file names and zstd) and `MemoryStore`
   (web; JS may persist entries in IndexedDB through the bridge). Implemented (P2.1) as
   `Options::cache: Option<&dyn TileStore>`, replacing upstream's `Option<&Path>`.
6. KTX2 supercompression: zstd under `native`; under `web` KTX2 is written uncompressed
   (supercompression scheme 0, already readable by `js/src/ktx2.ts`) or compressed by a pure-Rust
   encoder if one proves adequate (decided during Phase 2). **Decided (P2.1):** uncompressed on the
   web; a pure-Rust zstd encoder would add a dependency and still not match native bytes. Native
   `.lbake` bytes are unchanged.
7. Instance creation: on `web`, `Backends::BROWSER_WEBGPU` explicitly; adapter info and
   `isFallbackAdapter` are reported; fallback adapters are refused unless allowed.
8. Limits: request explicit limits (`maxStorageBufferBindingSize`, `maxBufferSize`) up to the
   adapter's, and validate scene sizes against them before allocation.

## Alternatives considered

- Keep the engine sync and run it with `wasm-bindgen-futures` + `Atomics.wait` on a worker:
  needs cross-origin isolation and still can't block on WebGPU callbacks. Rejected.
- Replace rayon with `wasm-bindgen-rayon`: needs SharedArrayBuffer/COOP+COEP and nightly target
  features; deferred to Phase 6 as an optional optimisation.

## Consequences

- The CLI keeps its behaviour; upstream tests become `async` (wrapped with `pollster`).
- Web bakes are single-threaded on the CPU side; CPU stages (filters, fills) are candidates for
  moving to GPU compute in Phase 6.

## Verification

`cargo build -p lightbaker --target wasm32-unknown-unknown --no-default-features --features web`
passes; upstream tests pass natively; the upstream test scene bakes in Chrome on WebGPU with the
same metrics as native (Phase 2 exit criteria).
