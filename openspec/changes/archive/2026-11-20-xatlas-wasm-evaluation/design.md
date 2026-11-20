# Design: xatlas-wasm-evaluation

## Context

`xatlas-wasm` 0.2.0: Emscripten build of unmodified xatlas (`f700c77`), ES module + sibling `xatlas.wasm`
(173 KB), `createXAtlas({ locateFile | wasmBinary })`, per-atlas `addMesh`/`computeCharts`/`packCharts`,
`getMesh(i)` returning `vertices: Vertex[]` (objects), `indices: Uint32Array`, `charts`. Worker protocol already
has an `unwrap` request in INTERFACES.md §7.

## Goals / Non-Goals

**Goals:** prove load/unwrap in the worker; validity checks reusable by P3.5/P3.6; numbers for ADR-0006.
**Non-Goals:** wiring into Rust (P3.6), caching by geometry hash (P3.7).

## Decisions

- **Adapter API mirrors the planned Rust `Unwrapper` contract** (ADR-0006 decision 1): input per tile is a list
  of meshes (positions, optional indices, normals, material ids); options are texels per metre, padding and
  chart options; output per mesh is `uv` (tile texels), `xref`, `indices`, `chart`, plus tile `width`/`height`.
  One `XAtlas` per tile, `resolution = 0`, `texelsPerUnit = texelsPerMeter`, `bilinear = true`,
  `faceMaterialData` from material ids; always `destroy()`.
- **Wasm URL:** the package's default loader (`new URL('xatlas.wasm', import.meta.url)`), which Vite emits as a
  worker asset; the package exports only its entry, so the `.wasm` cannot be imported with `?url`.
- **Validity checks** live in `app/src/bake/uvValidity.ts` (pure, typed arrays), shared by Vitest and the page.
  Flip check: sign of UV area per chart must be consistent (xatlas may mirror a whole chart).
- **Worker `unwrap`:** `readScene` → per mesh, non-indexed positions (v1 scenes) → `unwrapTile` → `unwrapped`
  with per-mesh `{ name, width, height, charts, utilisation, ms, uv, xref, indices }`. Lazy xatlas load on the
  first request (load time reported).
- **Measurements:** Node (Vitest logs: time per mesh, `process.memoryUsage().arrayBuffers`), browser
  (`app/scripts/hw-check.mjs --unwrap`: Chrome on the Intel machine, times per mesh from the page).

## Risks / Trade-offs

- [`Vertex[]` object output is slow for large meshes] → measured; if significant, ask upstream for typed-array
  getters or copy once in the adapter.

GPU-to-CPU readbacks, blocking calls: none. Native-only dependencies: none (xatlas-wasm is a web/Node package;
its Emscripten runtime has no threads). ADRs: ADR-0006 updated with the evaluation.
