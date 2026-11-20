# Design: browser-poc

## Context

`lightbaker-web` (`wasm-pack --target web`) exports `default` init, `capabilities`, `WebBaker`. ADR-0002:
baker in a dedicated module worker. No editor yet; ADR-0007 (React) applies from Phase 5.

## Goals / Non-Goals

**Goals:** smallest page proving S2 and serving P2.6's e2e; protocol code reusable by the Phase 5 editor.
**Non-Goals:** styling, React, runtime shading.

## Decisions

- **pnpm workspace** (`js`, `app`); the wasm package is consumed through a Vite alias to
  `crates/lightbaker-web/pkg` (generated, git-ignored) rather than a workspace dependency, so `pnpm install`
  works before the wasm is built. `pnpm -C app build:wasm` runs `wasm-pack`.
- **Testable worker:** `bakerCore.ts` exports `createBakerCore(bridge, post)`; the bridge interface mirrors
  `WebBaker` (`create`, `bake`, `cancel`, `evaluate`, `adapter`, `setDeviceLostCallback`). Vitest drives it
  with a fake bridge (progress → `progress` events with elapsed ms, rejection `{kind:'cancelled'}` →
  `cancelled` event, other kinds → `error`, concurrent bake → `error` `internal`). `client.ts` is tested with
  a fake `Worker`.
- **Protocol additions (documented in INTERFACES.md §7):** `bake` carries `expect?: string` (JSON) so the
  worker returns the check report with `done`; `init` carries `allowFallbackAdapter`; `done.result` is the
  P2.4 bridge output plus `checks`. Results' typed arrays are transferred.
- **View:** Three.js `WebGLRenderer` (the PoC only displays textures; renderer choice for the editor stays
  with P5.3). Meshes from `readScene`; lightmap UVs from `readBake` (`charts`, `triangleCharts`) +
  `lightmapUVs`; textures from the bridge's float arrays (`DataTexture`, `RGBAFormat`, `FloatType`, linear
  filtering only where float-linear is supported, else nearest). Shown with `MeshBasicMaterial`: modes
  *indirect* (tone-mapped irradiance), *sun field*, *AO*. No scene lights (never light twice: only the
  bake is displayed).
- **Scene source:** `test-scenes/upstream-basic/*` imported with Vite `?url` (fs.allow the repo root);
  `?profile=ci` selects a smaller bake (P2.6 tunes it).

## Risks / Trade-offs

- [Float textures not filterable on some devices] → check `OES_texture_float_linear`; fall back to nearest.
- [Worker module + wasm URL resolution in Vite] → `new Worker(new URL(...), { type: 'module' })` and
  wasm-pack's `--target web` `init(url)` with `?url` import.

GPU-to-CPU readbacks: none new (the page draws the arrays the bake already returns). Blocking calls: none.
Native-only dependencies: none. ADRs: none created (implements ADR-0002).
