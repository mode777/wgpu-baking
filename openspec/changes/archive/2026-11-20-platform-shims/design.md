# Design: platform-shims

## Context

ADR-0003 decides the split. Rayon is used in 5 places (`bake.rs` ×4, `deps.rs` ×1) only as
`par_iter()/par_iter_mut()` + `zip/map/filter/enumerate/collect`. `Cache` wraps an optional directory;
`Options::cache: Option<&Path>`. `encode::ktx2` always zstd-compresses (scheme 2). The workspace pins `wgpu = "30"`
with default features (all native backends + webgpu).

## Goals / Non-Goals

**Goals:** wasm32 build compiles; native bytes unchanged; minimal diffs in upstream files (imports and
cfgs). **Non-Goals:** making the web build *run* (needs async GPU, P2.2).

## Decisions

- **`par` module re-exports `rayon::prelude::*` under `native`; otherwise extension traits `par_iter()` /
  `par_iter_mut()` on slices returning `std::slice::Iter(Mut)`.** Upstream call sites keep their code; only
  `use rayon::prelude::*` → `use crate::par::*`. Results are identical (items are independent; `collect`
  preserves order in both). Alternative: `maybe_rayon` crate — an extra dependency for 20 lines.
- **`web-time::Instant` everywhere** (re-exports `std::time::Instant` natively), no cfg needed.
- **`TileStore` trait** `get(kind, hash) -> Option<Vec<u8>>`, `put(kind, hash, bytes) -> Result<()>`, `Sync`.
  `Cache<'a>` wraps `Option<&'a dyn TileStore>` so `bake.rs`/`volume.rs` call sites stay as they are.
  `FsStore::new(dir)` keeps upstream's `{kind}-{hex}.bin` zstd files (native only). `MemoryStore` is a
  `Mutex<BTreeMap>` (works on wasm32 single-threaded) with `len()`/`clear()`; persistence is left to the
  bridge. Changing `Options::cache` to the trait object (rather than a second field) keeps one way to cache.
- **KTX2 scheme by feature** in one private function `supercompress(data) -> (scheme, bytes)`; the header
  writes the returned scheme. Native: zstd level 19 (unchanged bytes). Web: scheme 0, raw. Alternative:
  pure-Rust `ruzstd` encoder on the web — adds a dependency and its output differs from native anyway;
  revisit if `.lbake` size matters (ADR-0003 point 6 now decided).
- **wgpu features:** workspace `wgpu = { version = "30", default-features = false }`; `lightbaker` enables
  `std`, `wgsl`, `parking_lot`; `native` adds `wgpu/default`; `web` adds `wgpu/webgpu`. WebGL is never
  enabled (no compute).
- **Native-only tests/examples** declare `required-features = ["native"]`, so
  `cargo test --no-default-features --features web --lib` can run the platform-independent unit tests
  natively (MemoryStore, KTX2 scheme 0, `par`).

## Risks / Trade-offs

- [`blake3`/`glam` features on wasm32] → verified by the wasm32 build task.
- [Web bakes CPU stages single-threaded] → accepted (ADR-0003); measured in P2.8.

GPU-to-CPU readbacks: none new. Blocking calls: none new; `pollster` becomes native-only (the remaining
`mpsc` readback is replaced in P2.2). Native-only dependencies: rayon, pollster, zstd now gated behind `native`.
ADRs: ADR-0003 updated (status, KTX2 decision, `Options::cache` type).
