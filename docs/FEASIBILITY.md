# Technical feasibility report

**Subject:** evolving [jer-nc/lightbaker](https://github.com/jer-nc/lightbaker) into a browser-based
WebGPU lightmap baker with xatlas UVs, general lights, emissive GI and a Three.js editor.
**Upstream inspected:** commit `4fe85da` (2026-11-19), workspace version 0.1.0, edition 2024,
`wgpu 30.0.1`. **Date:** 2026-11-20.

## 1. Verdict

**Feasible.** The core is small (≈ 100 KB of Rust/WGSL), well structured and already does the hard
parts (CPU BVH + WGSL compute traversal, multi-bounce gather, chart-aware filtering, tile cache,
irradiance volume, deterministic output). Nothing in the GPU code is native-only. The work splits
into:

| Area | Effort | Risk |
|---|---|---|
| WASM/WebGPU port (async GPU, platform shims) | medium | low–medium (well-understood blockers, §3) |
| xatlas in the browser | small–medium | **low–medium** (proven by `xatlas-wasm`; integration via JS adapter, ADR-0006) |
| Per-triangle UVs replacing affine charts | medium | medium (touches layout, raster, BVH, shader, output) |
| Point/spot/area lights, emissive, vertex albedo | medium | low (clear extension points, §6) |
| Editor app (React/Three.js) | large | low–medium (scope) |
| Witness-like quality (interpolation, denoise, seams, tuning) | large | medium (subjective target; measurable proxies in TESTING.md) |

Two assumptions in the brief are corrected (§8): the baker is not yet browser-capable, and the
Witness article URLs given are wrong.

## 2. Upstream inventory

| File | Size | Role | Change needed | Phase |
|---|---|---|---|---|
| `Cargo.toml` (workspace) | 0.6 KB | deps: anyhow, blake3, bytemuck, clap, glam 0.34, pollster, rayon, serde(_json), wgpu 30, zstd 0.14, flate2 | add `web`/`native` features, `web-time`, `futures-channel`, `wasm-bindgen*` (web crate), new members | 1–2 |
| `crates/lightbaker/src/lib.rs` | 0.8 KB | module list, re-exports | new modules (`platform`, `lights`, `store`) | 2–4 |
| `src/bake.rs` | 22 KB | orchestration: layout → hashes → cache → raster → Direct pass → CPU direct/AO/SDF → bounces (Gather) → volume → atlas assembly | async; progress/cancel; `TileStore`; lights pass; emission/vertex albedo materials; per-triangle UVs instead of `affines`; combined output | 2, 3, 4 |
| `src/gpu/mod.rs` | 9 KB | device, bind group layout (8 bindings), 3 pipelines, batched `run()` with readback | async open/readback; explicit limits; web instance; new pass; packed buffers (§5) | 2, 3, 4 |
| `src/gpu/trace.wgsl` | 12.6 KB | BVH traversal, Möller–Trumbore, `direct`, `gather`, `volume` entry points | `Hit` barycentrics; `bounced()` UV + emission + per-tri albedo; `lights` entry point; double-sided | 3, 4 |
| `src/bvh.rs` | 6.2 KB | binned SAH BVH, 32 B nodes, 48 B triangles (`v0.w` material, `e1.w` chart) | extend `Tri` with UVs/page/albedo (§5) | 3, 4 |
| `src/layout.rs` | 13 KB | planar charts by region growing, per-tile skyline packing, shelving, affine maps | becomes one `ChartSource`; shelving reused for xatlas object tiles; multi-page | 3 |
| `src/raster.rs` | 4.8 KB | conservative texel rasterisation through `Chart::local` | rasterise from per-triangle UVs; interpolated normals | 3 |
| `src/filter.rs` | 5.6 KB | edge-aware smooth, fill, sun SDF (CPU) | unchanged first; GPU à-trous later | 6 |
| `src/deps.rs` | 4.3 KB | tile hashing (sun-swept reach), `BAKER_VERSION = 11`, uses rayon | lights in hash; rayon shim; bump version | 2, 4 |
| `src/cache.rs` | 1.9 KB | fs + zstd tile cache | `TileStore` trait, `FsStore`/`MemoryStore` | 2 |
| `src/volume.rs` | 6.8 KB | L1 SH probe grid baked in chunks, `Instant` | time shim; emission via shared `bounced()` | 2, 4 |
| `src/encode.rs` | 6.6 KB | BC4/BC5, RGB9E5, KTX2 + zstd | zstd gated to native; RGBA16F output | 2, 5 |
| `src/output.rs` | 7.6 KB | `.lbake` writer, validity codes | combined-mode output; per-vertex UV chunk; v2 | 3, 5 |
| `src/scene.rs` | 8.2 KB | `.lbscene` v1 reader/writer, `deny_unknown_fields` | v2 (lights, emission, doubleSided, indexed meshes, normals, colours, material ids) | 3, 4 |
| `tests/bake.rs` | 8.3 KB | GPU integration test (shadow, AO, red bleed, volume, strip, cache determinism) | async; reused as browser parity test; `ground_texel` via UVs | 2, 3 |
| `crates/lightbaker-cli` | 2.4 KB | clap CLI | `block_on` wrapper; `FsStore` | 2 |
| `js/src/*.ts` | ~13 KB | scene writer, bake reader, `lightmapUVs` (affine), volume sampling, KTX2/BC5 decode | scene v2 writer; per-vertex UV reader; Three.js lightmap helper | 3, 5 |
| `docs/DESIGN.md`, `docs/FORMAT.md` | — | upstream design and formats | extend for v2 formats | 3–5 |

## 3. WASM blockers (exact locations)

| # | Location | Construct | Problem on `wasm32-unknown-unknown` + WebGPU | Fix (ADR-0003) |
|---|---|---|---|---|
| B1 | `gpu/mod.rs` `Gpu::new` | `pollster::block_on(Self::open())` | Cannot block the JS event loop | public `async fn open`; `new` only under `native` |
| B2 | `gpu/mod.rs` `Gpu::run` | `std::sync::mpsc::channel` + `device.poll(PollType::wait_indefinitely())` + `rx.recv()` | `poll` does not wait on the web; `recv()` blocks forever → deadlock | `map_async` + `futures` oneshot `.await`; native polls first |
| B3 | `gpu/mod.rs` `open` | `InstanceDescriptor::new_without_display_handle_from_env()` | Environment-driven backend selection | explicit `Backends::BROWSER_WEBGPU` on web; report `isFallbackAdapter` |
| B4 | `bake.rs` (`Instant::now` ×3+), `volume.rs` | `std::time::Instant` | Panics ("time not implemented") | `web-time` |
| B5 | `bake.rs` (`par_iter`, `par_iter_mut` ×5), `deps.rs` (`par_iter`) | `rayon` | No threads without SharedArrayBuffer/COOP+COEP | `par` shim: rayon on native, sequential on web |
| B6 | `cache.rs` | `std::fs::{create_dir_all, read, write, rename}` | No filesystem | `TileStore` trait |
| B7 | `cache.rs`, `encode.rs` | `zstd` (→ `zstd-sys`, C) | C library; needs a libc for wasm32 | gate to `native`; web: KTX2 without supercompression (already readable by `js/src/ktx2.ts`) |
| B8 | `bake()` call chain | synchronous API | Every GPU readback is async on the web | `bake` becomes `async` (CLI and tests use `block_on`) |
| B9 | `bake.rs` atlas-sized `Vec`s | `light: Vec<[f32;4]>` (16 B/texel), `direct_out`, `validity`, per-tile arrays | wasm32 heap ≤ 4 GiB, practical budget lower | budget check before allocation; pages; Phase 6: keep `light` on the GPU |

## 4. Dependency audit (lock file versions)

| Crate | Locked | Web status | Action |
|---|---|---|---|
| `wgpu` | 30.0.1 (latest 30.0.2) | `webgpu` backend supported; default features include it | web build: `default-features = false`, features `webgpu`, `wgsl`; no `webgl` for the baker |
| `glam` | 0.34.3 | pure Rust | none |
| `bytemuck` | 1.25.2 | pure Rust | none |
| `serde`, `serde_json` | 1.x | pure Rust | none |
| `anyhow` | 1.x | pure Rust | none (convert to `JsError` at the bridge) |
| `blake3` | 1.8.7 | portable Rust implementation on wasm32 (SIMD/asm paths are x86/ARM only) | verify build in Phase 2; optional `wasm32_simd` |
| `flate2` | 1.1.11 (miniz_oxide 0.9) | pure Rust backend | none |
| `rayon` | 1.12 / core 1.13 | not usable without threads | gate (B5) |
| `pollster` | 1.0.1 | blocks | gate (B1) |
| `zstd` | 0.14.3 (zstd-sys 2.1.2) | C dependency | gate (B7) |
| `clap` | 4 | CLI only | none |
| new: `web-time`, `futures-channel`, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`, `serde-wasm-bindgen`, `console_error_panic_hook`, `cc` (native xatlas); npm `xatlas-wasm` (web) | — | — | add in Phases 2–3 |

## 5. GPU pipeline analysis (WebGPU constraints)

**Bindings.** One bind group: `nodes`, `tris`, `materials`, `params` (uniform), `texels`, `out`,
`charts`, `light` → **7 storage buffers + 1 uniform**. WebGPU's default
`maxStorageBuffersPerShaderStage` is **8**. Adding separate buffers for lights, per-triangle UVs and
per-triangle albedo would exceed it. Plan:

- Extend `Tri` from 48 B to 80 B: `v0 (xyz, material)`, `e1 (xyz, flags)`, `e2`, `uv01 (u0,v0,u1,v1)`,
  `uv2 (u2, v2, page, packed albedo RGB9E5/unorm)`. This **removes** the `charts` buffer (ADR-0004).
- Lights in a **uniform** buffer (≤ 64 KiB: e.g. 256 lights × 64 B) or appended to `materials`.
- Result: 7 storage + 1–2 uniform, within defaults.

**Buffer sizes vs. default limits** (`maxStorageBufferBindingSize` 128 MiB, `maxBufferSize` 256 MiB):

| Buffer | Size | Fits default at |
|---|---|---|
| `light` (atlas, `vec4f`) | 16 B × W × H | ≤ 2896² (2048²: 64 MiB ✓; 4096²: 256 MiB ✗) → request higher limit or split per page / use `rgba16float` texture + `textureLoad` |
| `texels` (64 B each; bound per batch slice) | 64 B × covered texels | buffer ≤ 4 M texels at 256 MiB; bound slices small ✓ |
| `tris` (80 B after change) | 80 B × casting triangles | 1.6 M triangles per 128 MiB ✓ |
| `nodes` (32 B) | ≈ 2 × triangles / 4 × 32 B | ✓ |
| `out` per batch | 16 B × 65 536 = 1 MiB | ✓ |

Texel slices are bound at offsets `first × 64` with `first` a multiple of the batch size → aligned
to `minStorageBufferOffsetAlignment` (256) ✓.

**Readbacks.** Every batch reads back its results (Direct: per 65 536 texels; Gather: per 16 384
texels per bounce; Volume: per 8 192 probes), and the `light` atlas is re-uploaded each bounce.
Acceptable for Phase 2 (correctness first). Phase 6 target: keep `light` and gather results on the
GPU, run fill/smooth as compute passes, read back only the final atlas (REQ-WASM-05).

**Watchdog.** Upstream already batches to avoid driver resets. Browser GPU watchdogs (and Windows
TDR ≈ 2 s) apply too; batch sizes become adaptive (measure dispatch time, shrink on slow GPUs).

**WGSL portability.** The shader uses only core WGSL (private arrays, `reverseBits`, `bitcast`,
`select`), no naga extensions. It must be compiled by the browser (Tint in Chrome) — verified in
the Phase 2 spike; uniformity-analysis differences are the main risk.

## 6. Extension points for lighting

| Feature | Where | How |
|---|---|---|
| Point/spot/area lights | `scene.rs` (`lights`), `gpu/mod.rs` (`Pass::Lights`), `trace.wgsl` (new `lights` entry), `bake.rs` (direct assembly next to the sun's `direct` vec) | per texel: loop lights, sample, shadow ray with `trace(…, any = true)`; result added to `TileLight.direct` → automatically bounced |
| Emissive materials | `scene.rs` `Material.emission`; `bake.rs` materials buffer; `trace.wgsl::bounced()` | add `Le` on front-face hits; volume pass shares `bounced()` |
| Vertex-colour albedo | scene v2 `colors`; `bvh.rs` `Tri`; `bounced()` | per-triangle albedo overrides material albedo |
| Double-sided | `Material.double_sided`; `trace()` `back` flag; `inside()` | treat back hits as front, exclude from inside test |
| Lights in tile hashes | `deps.rs` `global_hash` | first: all lights in the global hash (any light change rebakes all); later: range-limited per tile |
| Sun as SDF | unchanged | kept for `stationary-sun` output |

## 7. UV/chart analysis

Upstream charts are **planar with one affine world→atlas map per chart**; the map is used by
`raster.rs`, by `trace.wgsl::bounced()` (`charts` buffer, `Tri.e1.w` = chart) and by `js/uv.ts`.
Tiles are **ground-plane squares** (`tile_of(p)` on x/z), suited to large outdoor maps. xatlas
produces non-planar charts with per-vertex UVs and seams. Therefore (ADR-0004): per-triangle UVs on
the GPU, UV-space rasterisation, object tiles packed by xatlas and shelved by upstream's
`shelve()`, multiple pages. Upstream's planar layout feeds the same path.

Also relevant: upstream meshes are **non-indexed world-space triangles, one material per mesh, face
normals only**. Scene v2 adds indices, normals, per-triangle material ids and colours.

## 8. Corrections to the brief's assumptions

1. *"Its architecture is already designed around browser technologies and WebGPU … a
   TypeScript-facing interface."* — The runtime that **consumes** bakes is a three.js/WebGPU game;
   the **baker** runs natively (README: "wgpu: Vulkan, DX12, Metal"). The TS package writes scenes
   and reads bakes only. The browser port is real work (§3), though unobstructed.
2. The three `blog.selfshadow.com/publications/…` URLs return 404; correct sources are in
   [RESEARCH.md §2](RESEARCH.md#2-the-witness-lighting-articles-ignacio-castaño-jonathan-blow).
3. Castaño's released hemicube/irradiance-caching code is a Gist **without a licence**: reference
   only.
4. polyfork.dev assets are vertex-coloured with one shared material → per-vertex albedo is a
   requirement the brief did not anticipate (REQ-LGT-10).

## 9. Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `xatlas-wasm` (young, single maintainer) breaks or is abandoned | low–medium | medium | pin exact version; thin adapter; fall back to `watlas` or a self-built Emscripten module from the same pinned xatlas commit (ADR-0006) |
| Software adapters in CI (lavapipe) hide real-GPU bugs or are too slow | medium | low–medium | CI-sized scenes; real-GPU runs in phase reports; optional `gpu-perf` job (ADR-0009) |
| Browser WGSL compiler rejects/miscompiles the shader | low | high | Phase 2 spike first; keep core WGSL; CI on Chrome |
| WebGPU limits too low for High preset on some GPUs | medium | medium | explicit limit requests, pages, budget checks, clear errors; Phase 6 textures instead of buffers |
| GPU watchdog / device loss on long dispatches | medium | medium | adaptive batch sizes; device-lost recovery (REQ-WASM-07) |
| Readback-heavy pipeline slow in browsers | medium | medium | measure in Phase 2; move fills/filters to GPU in Phase 6 |
| Single-threaded CPU stages slow (BVH, xatlas, filters) | medium | medium | object-tile caching; GPU filters; optional wasm threads later |
| Three.js lightmap semantics differ from assumptions | low | medium | Phase 5 calibration test pins the version |
| polyfork terms change | low | low–medium | live CDN only, nothing committed; GLB importer + primitives as first-class |
| Upstream evolves incompatibly | medium | low | additive, feature-gated changes; merge at phase boundaries |
| "Witness-like" quality is subjective | high | medium | measurable proxies (bleed ratios, bounce convergence, leak tests) + art direction review |

## 10. Phase 1 & 2 change list (what the first two phases touch)

**Phase 1 (fork, baseline):** import upstream with history; add workspace docs/CI; run upstream
tests natively on a GPU; record bake timings of the upstream test scene; no engine code changes.

**Phase 2 (WASM/WebGPU proof of concept):**

- `crates/lightbaker/Cargo.toml`: features `native` (default) / `web`; optional rayon, pollster,
  zstd; add `web-time`, `futures-channel`.
- `src/gpu/mod.rs`: B1–B3, explicit limits, adapter report.
- `src/bake.rs`, `src/volume.rs`, `src/deps.rs`: B4, B5, B8; progress + cancel hooks in `Options`.
- `src/cache.rs`: B6 (`TileStore`); `src/encode.rs`: B7.
- `crates/lightbaker-cli/src/main.rs`, `tests/bake.rs`: `block_on` around the async API.
- New `crates/lightbaker-web`: wasm-bindgen API (INTERFACES.md §8), capability check.
- New minimal `app/` page (Vite + worker, no editor): runs the upstream test scene in the browser,
  shows adapter info and the resulting lightmap textures, compares metrics with native.
