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

1. **Crate `lightbaker-uv`** defines the `Unwrapper` contract used by the Rust pipeline
   (input per object tile: world-space positions, normals, indices, material ids; options: texels
   per metre, padding, chart options; output: per-vertex UVs in tile texels, `xref`, new indices,
   chart ids, tile rectangle size).
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
