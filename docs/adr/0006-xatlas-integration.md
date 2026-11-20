# ADR-0006: xatlas integration — native FFI, `xatlas-wasm` in the browser

- **Status:** accepted (revised 2026-11-20 after evaluating `mode777/xatlas-wasm`)
- **Date:** 2026-11-20
- **Phase:** 3
- **Requirements:** REQ-UV-01 … 09, REQ-WASM-01, REQ-WASM-03

## Context

The brief asks to integrate xatlas into the Rust baking pipeline. Findings
([RESEARCH.md §3](../RESEARCH.md#3-xatlas)):

- Rust crates (`xatlas-rs`, `xatlas-rs-v2`) compile `xatlas.cpp` with `cc` + `bindgen` for native
  targets only; `wasm32-unknown-unknown` has no libc/libc++.
- **[`mode777/xatlas-wasm`](https://github.com/mode777/xatlas-wasm)** (MIT, npm `xatlas-wasm` 0.2.0,
  2026-11-09) already runs xatlas in browsers, Node and **Web Workers**: Emscripten 3.1.64 build of
  **unmodified** xatlas (`XATLAS_C_API=1`, `XA_MULTITHREADED=0`, C++17, `-O2 -flto`), separate
  `xatlas.wasm` (~250 KB unpacked package) loaded on demand, Vite-compatible asset handling
  (tested in its CI), TypeScript declarations. API: `addMesh` (positions, indices, normals, UVs,
  `faceMaterialData` so charts never span materials), `computeCharts`, `packCharts` (repeatable
  without recomputing charts; `resolution`, `padding`, `texelsPerUnit`, …), per-vertex `uv`,
  `xref`, `atlasIndex`, `chartIndex`, chart face arrays, utilisation, optional texel image,
  progress callback. Its showcase includes multi-model and multi-atlas scenarios.
- Its xatlas submodule (`mode777/xatlas` @ `f700c77`) is **identical to `jpcy/xatlas` master**
  (0 commits ahead/behind), so native and web can run the same xatlas source.
- The same author published `mode777/lightmap-studio` (Unlicense), an unwrap + bake studio built on
  it — prior art showing the approach works end to end in a browser.

## Decision

1. **The engine (`lightbaker::unwrap`)** defines the `Unwrapper` contract used by the Rust pipeline
   (input per object tile: world-space positions, normals, indices, material ids; options: texels
   per metre, padding, chart options; output: per-vertex UVs in tile texels, `xref`, new indices,
   chart ids, tile rectangle size) and the UV validity checks. (Originally planned in `lightbaker-uv`;
   moved so `lightbaker` keeps depending on no crate of ours, P3.5.) `lightbaker-uv` holds the native
   implementation.
2. **Native:** own small FFI wrapper over the xatlas C API, compiled with `cc` from a **vendored
   copy of `jpcy/xatlas` at `f700c77`** (MIT licence kept). Hand-written `extern "C"` declarations
   (no `bindgen`/libclang).
3. **Web:** `xatlas-wasm` (pinned exact version) is loaded **in the baker worker** at `init`;
   `lightbaker-web` imports a small JS adapter (`app/src/bake/xatlasUnwrapper.ts`, via a
   `wasm-bindgen` `extern` block) implementing the same contract. After the async module load all
   calls are synchronous, so the Rust pipeline calls it like a normal function. Data crosses as
   typed arrays (one copy each way per object tile).
4. **Object tiles** (ADR-0004): one xatlas `Atlas` per object tile (`resolution = 0` → auto-sized,
   `texelsPerUnit = texelsPerMeter`, `padding` from settings); the Rust side shelves tile
   rectangles into pages. Chart results are cached by a hash of (geometry, scale, options).
5. Upstream's **planar** layout stays available and remains the default until Phase 3 completes.
6. A freestanding build of xatlas inside the `wasm32-unknown-unknown` binary is **not pursued**
   (would need libc/libm shims and a patch for its unconditional threading includes). Revisit only
   if the cross-module copy becomes a measured bottleneck.

## Alternatives considered

| Option | Why not |
|---|---|
| Freestanding xatlas inside the Rust wasm binary | Unproven; shims and patches; no benefit that justifies the risk. |
| `watlas` (toji, MIT) | Viable backup with a close-to-native API; `xatlas-wasm` has more recent maintenance plus worker and Vite tests. |
| Build our own Emscripten module | Unnecessary while `xatlas-wasm` fits; fallback option (its CMake setup is MIT and small). |
| Port xatlas to Rust | ~10 k lines of geometry code; out of scope. |

## Consequences

- No LLVM/clang or Emscripten toolchain in our build: the web path is an npm dependency; native
  uses the platform C++ compiler.
- **Dependency risk:** young project, single maintainer. Mitigation: pin the exact version, keep the
  JS adapter thin, fall back to `watlas` or a self-built module from the same pinned xatlas commit.
- **Determinism:** native (MSVC/gcc/clang) and web (Emscripten `-O2 -flto`) builds may differ in
  floating-point details. Each target must be deterministic with itself; native vs web results
  must be *equivalent* (same chart count, valid UVs, UVs within a small epsilon), not bit-identical.
- `xatlas-wasm` caveats the adapter must respect: indices only as `Uint16Array`/`Uint32Array`
  (XW-006); `getUtilization(atlasIndex)` not bounds-checked (XW-010); call `destroy()` to free
  memory.

## Verification

P3.1 (evaluation, ≤ 1 day): load `xatlas-wasm` in our baker worker under Vite, unwrap the test
meshes per object tile, confirm the options and outputs ADR-0004 needs, record timings and memory.
P3.6: the UV validity suite passes natively and in the browser, with native/web equivalence checks.

## Evaluation results (P3.1, 2026-11-20)

`xatlas-wasm@0.2.0` (exact), adapter `app/src/bake/xatlasUnwrapper.ts`, validity checks
`app/src/bake/uvValidity.ts`, tests `app/test/xatlas.test.ts`, browser run
[docs/reports/phase-3/xatlas-eval-chrome-intel.json](../reports/phase-3/xatlas-eval-chrome-intel.json).

- **Loads in the baker worker under Vite** with the package's default loader (`new URL('xatlas.wasm',
  import.meta.url)`): Vite emits `xatlas.wasm` (173 KB, 71 KB gzip) as a worker asset. The package only
  exports its entry point, so `xatlas-wasm/dist/xatlas.wasm?url` cannot be imported — not needed.
  Load time: 26 ms in Chrome's worker (Intel machine), 4–6 ms in Node.
- **Options ADR-0004 needs are all there:** one atlas per tile, `resolution = 0` (auto-sized),
  `texelsPerUnit = texelsPerMeter`, `padding`, `bilinear`, `faceMaterialData` (charts never span
  materials — tested), chart options. Outputs: per-vertex UV in **atlas texels** (not normalised), `xref`,
  new `indices` (same triangle order as the input), `chartIndex`, atlas width/height, utilisation.
- **Validity** (out-of-range, degenerate, flipped, `xref` positions, mixed-material charts, density ±20 %):
  passes for a unit quad (33×33 tile at 32 texels/m, padding 2), a 3-material box, the four
  `upstream-basic` meshes, a 64×32 UV sphere and a 256×32 torus knot. Charts smaller than 4×4 texels
  at the requested density are excluded from the density check (xatlas gives every chart ≥ 1 texel:
  the 5 cm strip at 4 texels/m becomes a 2×11 tile).
- **Deterministic** with itself (two unwraps → identical UVs); Chrome and Node give identical tile sizes.
- **Timings** (Node, same wasm; Chrome warm is similar):

  | Mesh | Triangles | Charts | Tile | Utilisation | Time | of which charts |
  |---|---:|---:|---|---:|---:|---:|
  | upstream-basic ground (4 texels/m) | 512 | 1 | 257×257 | 100 % | 7 ms | 1 ms |
  | upstream-basic box / wall | 12 | 6 | 35×22 / 60×81 | 100 / 96 % | < 1 ms | — |
  | UV sphere 64×32 (64 texels/m) | 3 968 | 9 | 302×323 | 64 % | 125 ms | 113 ms |
  | torus knot 256×32 (64 texels/m) | 16 384 | 32 | 615×617 | 48 % | 1 929 ms | 1 890 ms |

  Browser (Chrome, worker): whole `upstream-basic` scene 43 ms cold (incl. 26 ms load), 6 ms warm.
- **Memory:** no measurable growth of Node's `arrayBuffers` for the small meshes, +0.3 MiB for the torus
  knot; `destroy()` after each tile releases the atlas. Output copies are typed arrays (`uv` 8 B,
  `xref` 4 B, `chart` 4 B per vertex; indices 12 B per triangle).
- **Caveats for the integration:**
  - **Chart computation dominates and grows super-linearly** (16 k triangles ≈ 1.9 s, single-threaded):
    caching charts per object tile by geometry hash (decision 4) is required for interactive rebakes;
    large meshes should be split into tiles; Phase 5 must show unwrap progress.
  - `getMesh().vertices` is an array of objects; copying it into typed arrays costs < 1 ms per 10 k
    vertices — not a bottleneck.
  - Utilisation of curved meshes is 48–64 % with the default (non-brute-force) packing; brute force is an
    option for final bakes (`xatlas.bruteForcePacking`).

**Conclusion:** keep the decision; no fallback (watlas / own Emscripten build) needed.

## Native implementation (P3.5)

`crates/lightbaker-uv`: `vendor/xatlas/` (`xatlas.cpp`, `xatlas.h`, `xatlas_c.h`, `LICENSE`, unmodified
`jpcy/xatlas` @ `f700c77`), `build.rs` compiling it with `cc` as C++17 with `XATLAS_C_API=1`,
`XA_MULTITHREADED=0` (as `xatlas-wasm`) and `-O2` even in debug builds; nothing is compiled for wasm32.
Hand-written `#[repr(C)]` mirrors in `src/ffi.rs`, checked against the C++ compiler's `sizeof` by a test
(`src/probe.cpp`). `XatlasUnwrapper` implements `lightbaker::unwrap::Unwrapper`: one atlas per tile, chart
options only override xatlas' defaults when set, packing with `resolution = 0`, `bilinear`, the requested
padding and `texelsPerUnit`; the atlas is destroyed however the call ends; more than one atlas is an error.

Validity suite (`crates/lightbaker-uv/tests/validity.rs`): a 2 m quad (one chart, tile 32–40 texels at
16 texels/m), a box with three materials (no mixed charts), each `upstream-basic` mesh and all of them in one
tile, a UV sphere, a 16 384-triangle torus knot (unwrapped twice: identical) — all checks pass and median
chart density is within ±20 % of the request (upstream-basic's sub-texel strip has no chart big enough to
measure). The torus knot takes ≈ 2 s natively, like the wasm build: chart computation dominates on both.
