# Research notes

Findings gathered on 2026-11-20 for the planning phase. Every claim has a source; claims that could
not be verified are marked **UNVERIFIED**. Re-check volatile facts (versions, browser support,
licences) at the start of the phase that depends on them.

## 1. Corrections to the brief

- The three article URLs in the brief (`blog.selfshadow.com/publications/precomputed-lighting/`,
  `/hemicube/`, `/irradiance-caching/`) **return 404**. `blog.selfshadow.com` is Stephen Hill's site
  and unrelated. The correct articles are listed in §2.
- The brief states lightbaker's "architecture is already designed around browser technologies and
  WebGPU" and has "a TypeScript-facing interface". Inspection shows the **baker itself is native
  only** (wgpu on Vulkan/DX12/Metal, `pollster`, `rayon`, filesystem cache). The `js/` package is a
  dependency-free scene **writer** and bake **reader**; it does not run the baker. The *consumer*
  (Orbitoon's game) uses three.js with WebGPU/WebGL. See [FEASIBILITY.md](FEASIBILITY.md).

## 2. The Witness lighting articles (Ignacio Castaño, Jonathan Blow)

| Article | URL | Key points |
|---|---|---|
| Precomputed Lighting (Blow, 2010) | https://the-witness.net/news/2010/03/graphics-tech-precomputed-lighting/ | Baked GI chosen over real-time middleware; render the scene from each lightmap texel (hemicube), integrate to irradiance, iterate bounces until convergence. |
| Lightmap Parameterization (Castaño, 2010) | https://the-witness.net/news/2010/03/graphics-tech-texture-parameterization/ | Automatic chart segmentation, parameterisation and packing — the lineage of `thekla_atlas` → **xatlas**. |
| Hemicube Rendering and Integration (2010) | https://the-witness.net/news/2010/09/hemicube-rendering-and-integration/ | Hemicube vs. other projections; atlas packing of hemicube faces; integration weights; near-plane/overlap issues. |
| Irradiance Caching, Part 1 (2011) | https://the-witness.net/news/2011/07/irradiance-caching-part-1/ | Ward's lazy irradiance evaluation; split-sphere record radius; RBF interpolation; translation/rotation **irradiance gradients**; references the SIGGRAPH'08 IC course and PBRT. |
| Irradiance Caching, Continued (2014) | Wayback: `http://www.ludicon.com/castano/blog/2014/07/irradiance-caching-continued/` (original domain offline) | Record placement refined by neighbour clamping / Hessian-style error control (Křivánek et al.; Schwarzhaupt, Jarosz, Jensen); links the released code. |

**Released code:** GitHub Gist https://gist.github.com/castano/5f7c8847f82e45cb56d0 (`hemicube.h`,
`hemicube.cpp`, `hemicube_integrate.cpp`, C++): hemicube face layout and integration weights,
precomputed data to evaluate irradiance and its gradients. **No licence is stated** → treat as
reference reading only; do not copy code into this repository.

**What we adopt** (ADR-0005): the *ideas* — gradient/error-driven adaptive sample placement,
harmonic-mean-distance validity radii, restricted interpolation, iterated bounces — on top of
lightbaker's ray-traced gather. We do not implement hemicube rasterisation.

## 3. xatlas

- Canonical: https://github.com/jpcy/xatlas — C++, MIT, "cleaned up version of thekla_atlas"; last
  push 2024-06. Predecessor: https://github.com/Thekla/thekla_atlas (MIT).
- **Source inspection (this session, `source/xatlas/xatlas.cpp`, 10 045 lines):** no STL containers
  (own `Array`, hash maps), allocation through `SetAlloc(realloc, free)` hooks and placement `new`,
  output through `SetPrint`. However it **unconditionally includes** `<atomic>`, `<condition_variable>`,
  `<mutex>`, `<thread>` and defaults to `XA_MULTITHREADED 1`; it needs `<math.h>`, `<string.h>`,
  `<stdio.h>` (printf/sprintf; `fopen` only in debug exports). → A freestanding
  `wasm32-unknown-unknown` build looks *plausible* with `XA_MULTITHREADED 0`, guarded includes and
  libc/libm shims provided from Rust, but has not been demonstrated (not pursued, see ADR-0006).

### `mode777/xatlas-wasm` (chosen for the browser)

Inspected 2026-11-20 (repository, `README.md`, `BUILD.md`, `CMakeLists.txt`, `API.md`, CI workflow):

- MIT; npm `xatlas-wasm` 0.2.0 (2026-11-09), ~250 KB unpacked; ESM-only, TypeScript declarations.
- Emscripten **3.1.64** (pinned and asserted in CI), CMake; xatlas compiled **unmodified** with
  `XATLAS_C_API=1`, `XA_MULTITHREADED=0`, C++17, `-O2 -flto`; `MODULARIZE`, `EXPORT_ES6`,
  `ALLOW_MEMORY_GROWTH`, `ENVIRONMENT=web,node,worker`.
- Separate `dist/xatlas.wasm` loaded on demand (`locateFile`/`wasmBinary` options); Vite consumer
  build tested in CI (emits the `.wasm` asset); runs inside Web Workers (single-threaded kernel).
- API: `createAtlas`, `addMesh` (positions, indices, normals, UVs, `faceMaterialData`),
  `addUvMesh`, `computeCharts`, `packCharts` (repeatable without recomputing charts), `generate`,
  `setProgressCallback`, `getMesh` (per-vertex `uv`, `xref`, `atlasIndex`, `chartIndex`), charts with
  face arrays and material, `width`/`height`/`atlasCount`/`utilization`/`texelsPerUnit`,
  optional `getImage()` texel map, `destroy()`.
- Submodule `mode777/xatlas` @ `f700c77` — **identical to `jpcy/xatlas` master** (GitHub compare:
  0 ahead, 0 behind).
- Known caveats (its backlog): indices must be `Uint16Array`/`Uint32Array` (XW-006);
  `getUtilization` not bounds-checked (XW-010).
- Live showcase: https://mode777.github.io/xatlas-wasm/ (multi-model town, multi-atlas stress cases).

### Rust bindings

| Crate | Version / date | Notes |
|---|---|---|
| `xatlas-rs` | 0.1.3, 2019-06 | Unmaintained. MIT. |
| `xatlas-rs-v2` | 0.1.4, 2025-11 | Republished fork of the above; `build.rs` compiles `xatlas.cpp` with `cc` (`-std=c++11`) and runs `bindgen`. No wasm support or CI found. MIT. |

### xatlas compiled to WebAssembly (npm, all Emscripten/wasi builds, all MIT)

| Package | Version / date | Notes |
|---|---|---|
| `xatlas-wasm` (mode777) | 0.2.0, 2026-11-09 | Actively maintained, ESM, Node/browser/workers, single-threaded. |
| `watlas` (toji) | 1.0.1, 2025-07 | Close to the native API; used by glTF-Transform. Emscripten ≥ 4.0.9. |
| `xatlasjs` / `xatlas-three` (repalash) | 0.2.x, 2024-09 | Worker wrapper (Comlink), three.js helper. |
| `xatlas-wasi` (kzhsw) | 0.0.2, 2024-07 | wasi-sdk build of the raw C API. |
| `xatlas-web` (MozillaReality) | 0.1.0, 2020 | Inactive. |

Related prior art: `mode777/lightmap-studio` (Unlicense, 2026-11-11) — browser unwrap + bake studio
built on `xatlas-wasm`; worth reviewing for UX and pitfalls (not a code source for us unless its
licence is re-checked).

→ Decision: [ADR-0006](adr/0006-xatlas-integration.md) — native FFI over the same pinned xatlas
commit, `xatlas-wasm` in the baker worker.

## 4. wgpu and WebGPU

- **wgpu** latest: **30.0.2** (2026-11-12), MIT OR Apache-2.0, MSRV 1.87. Upstream lightbaker pins
  `wgpu = "30"` (lock: 30.0.1). Default features include `webgpu`; `webgl` is separate.
- **Default (WebGPU-guaranteed) limits** (`wgpu-types/src/limits.rs`): `maxStorageBufferBindingSize`
  128 MiB, `maxBufferSize` 256 MiB, `maxComputeWorkgroupStorageSize` 16 KiB, uniform binding 64 KiB.
  Higher limits must be requested from the adapter and may not be available.
- Optional features on the web include `TIMESTAMP_QUERY` and `IMMEDIATES` (push-constant-like);
  neither may be assumed.
- On the web, buffer mapping completes asynchronously via the browser; `device.poll(wait)` cannot
  block (well-established wgpu/WebGPU behaviour; no single source line cited — high confidence).
- **Browser support** (caniuse snapshot 2026-11-20): Chrome/Edge ≥ 113 (desktop), Chrome Android,
  Opera, Samsung Internet: supported. Safari 26+: supported on iOS, partial on macOS. Firefox:
  listed as **not enabled by default** in the snapshot — support varies by platform and release;
  **re-verify** in the Phase 6 browser matrix. See also https://github.com/gpuweb/gpuweb/wiki/Implementation-Status.

### GPU testing in GitHub Actions (checked 2026-11-20)

- Standard GitHub-hosted runners have **no GPU**. GPU larger runners exist (4 vCPU, 1 × Tesla T4,
  16 GB VRAM, Ubuntu/Windows), billed per minute
  (https://docs.github.com/en/actions/reference/runners/larger-runners).
- **wgpu's CI** runs its GPU tests on standard runners with software drivers: Mesa **lavapipe**
  on Linux (`install-mesa` action, ICD via `VK_DRIVER_FILES`), **WARP** and Mesa on Windows, Metal
  on macOS (`gfx-rs/wgpu/.github/workflows/ci.yml`).
- **three.js** runs its WebGPU examples in Chrome on `ubuntu-latest` after
  `apt-get install mesa-vulkan-drivers xvfb`, headed under `xvfb-run`, with
  `--enable-unsafe-webgpu --enable-features=Vulkan --disable-vulkan-surface --ignore-gpu-blocklist
  --disable-gpu-watchdog --no-sandbox` (`mrdoob/three.js` `ci.yml`, `test/e2e/puppeteer.js`). Some
  heavy examples are excluded there for time or rasteriser discrepancies.
- → Decision: [ADR-0009](adr/0009-ci-github-actions.md).

## 5. polyfork.dev

Summary only; details, licence terms and the integration plan are in
[ASSET_LIBRARY.md](ASSET_LIBRARY.md).

- Library of ~3 500 free **low-poly, flat-shaded, vertex-coloured** assets (single shared
  material, no textures, 1 unit = 1 m) plus paid HD editions.
- Public REST API (`https://polyfork.dev/api`, documented at `/llms.txt`), MCP server, CDN GLBs at
  `https://polyfork.dev/cdn/{id}.glb` with `access-control-allow-origin: *`.
- Free mesh files: **CC BY 4.0** (credit required) or the Polyfork licence (no credit, no
  redistribution). Hotlinking the free GLBs from the CDN per use is explicitly allowed.
- Project decision (2026-11-20): polyfork meshes are generated by scripts; committing them to a
  public repository could be seen as redistribution → **never commit them**; link the CDN only.

## 6. Three.js lightmap conventions (to verify in Phase 5)

- Lightmaps select their UV set via `texture.channel` (`uv1` for channel 1) — pin with the Phase 5
  calibration test.
- Physically based lights (no legacy mode): point/spot `distance` window and decay 2 as described
  in [LIGHTING_MODEL.md §3](LIGHTING_MODEL.md#3-light-sources). The exact shader code of the pinned
  Three.js version is checked when the calibration test is written.
