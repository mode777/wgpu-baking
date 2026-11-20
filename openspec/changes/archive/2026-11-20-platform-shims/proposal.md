# Proposal: platform-shims (P2.1)

## Why

`lightbaker` does not compile for `wasm32-unknown-unknown`: it uses rayon, `std::time::Instant`, the file
system and the C library zstd unconditionally (FEASIBILITY.md blockers B4–B7). Phase 2's first success
criterion (S1) is that the engine compiles to wasm32. This change makes the CPU-side platform dependencies
switchable without changing native behaviour. Requirements: REQ-WASM-01, REQ-WASM-03, REQ-WASM-10.

## What Changes

- Cargo features on `lightbaker`: `native` (default: rayon, pollster, zstd, file cache, all native wgpu
  backends) and `web` (wgpu `webgpu` + `wgsl` only, no rayon/zstd/fs). The workspace `wgpu` dependency drops
  its default features so each crate opts in.
- `par` shim: rayon's `par_iter`/`par_iter_mut` under `native`, sequential iterators otherwise (B5).
- Time: `web-time`'s `Instant` replaces `std::time::Instant` (B4).
- **BREAKING (engine API):** `Options::cache` becomes `Option<&dyn TileStore>`. `TileStore` has `FsStore`
  (native; upstream file names and zstd, unchanged on-disk format) and `MemoryStore` (any platform) (B6).
- zstd only under `native`: under `web`, KTX2 textures in `.lbake` are written without supercompression
  (scheme 0), which `js/src/ktx2.ts` already reads (B7). Native output bytes are unchanged.
- `Gpu::new` (blocking, pollster) only under `native`.

## Capabilities

### New Capabilities

- `engine-platforms`: which platforms the engine builds for, what each feature set provides, and the
  pluggable tile store.

### Modified Capabilities

None.

## Out of scope

Async GPU access and readback, web instance/adapter selection and limits (P2.2, B1–B3, B8), CI wasm job
(P2.3), the wasm-bindgen bridge (P2.4), IndexedDB persistence of `MemoryStore` (later), pure-Rust zstd.

## Impact

`Cargo.toml` (workspace), `crates/lightbaker` (`Cargo.toml`, new `par.rs`, `cache.rs`, `encode.rs`,
`bake.rs`/`deps.rs`/`volume.rs` imports, `gpu/mod.rs` cfg), CLI and tests (FsStore), ADR-0003, BUILD.md,
THIRD_PARTY_NOTICES.md.
