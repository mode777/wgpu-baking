# AGENTS.md

Guidance for AI coding agents (and humans) working in this repository. Read this file completely
before changing anything.

## What this project is

A browser-based, GPU-accelerated **lightmap baker and lightweight 3D scene editor**. It is a fork of
[jer-nc/lightbaker](https://github.com/jer-nc/lightbaker) (Rust + wgpu + WGSL), compiled to
`wasm32-unknown-unknown` and run on **WebGPU** in the browser, with automatic lightmap UVs from
**xatlas**, a general lighting model (sun, sky, point, spot, area, emissive) and a **React + Three.js**
editor. Visual target: stylized, fully baked, multi-bounce diffuse GI with strong colour bleeding,
reminiscent of *The Witness* (2016).

The full brief is in [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md). The numbered requirements derived
from it are in [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md).

## Current status

**Planning complete, implementation not started.** No source code has been forked or written yet.
The active phase and its exit criteria are tracked in [ROADMAP.md](ROADMAP.md).

## Document map (source of truth)

| Topic | Document |
|---|---|
| Phases, milestones, exit criteria, risks | [ROADMAP.md](ROADMAP.md) |
| Requirements with IDs (`REQ-…`) | [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) |
| Inspection of upstream lightbaker; exact files and dependencies to change | [docs/FEASIBILITY.md](docs/FEASIBILITY.md) |
| Target architecture, crate boundaries, data flow | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| Radiometry, light types, GI algorithm, energy conservation | [docs/LIGHTING_MODEL.md](docs/LIGHTING_MODEL.md) |
| TypeScript/Rust interfaces, worker protocol, scene format v2 | [docs/INTERFACES.md](docs/INTERFACES.md) |
| Test strategy, deterministic scenes, e2e budget, CI | [docs/TESTING.md](docs/TESTING.md), [ADR-0009](docs/adr/0009-ci-github-actions.md) |
| Showcase and test scenes (Tests 1–5) | [docs/DEMO_SCENES.md](docs/DEMO_SCENES.md) |
| Asset library / polyfork.dev findings | [docs/ASSET_LIBRARY.md](docs/ASSET_LIBRARY.md) |
| External research (The Witness, xatlas, WebGPU limits) | [docs/RESEARCH.md](docs/RESEARCH.md) |
| Toolchain and build instructions | [docs/BUILD.md](docs/BUILD.md) |
| Architecture decisions | [docs/adr/](docs/adr/) |

If code and documents disagree, stop and resolve the disagreement (fix the code or update the
document in the same change). Never leave them inconsistent.

## Workflow

This repository uses **OpenSpec** (`openspec/`, CLI `openspec`, prompts in `.github/prompts/opsx-*`).

1. Pick the next unchecked item of the **current phase** in [ROADMAP.md](ROADMAP.md). Do not start
   items from a later phase. In particular: **no work on Phase 3+ until the Phase 2 exit criteria
   (the existing baker baking a scene in a browser on WebGPU) are met and recorded.**
2. Create an OpenSpec change for it (`/opsx-propose`, change name = the roadmap's suggested name).
3. Implement with `/opsx-apply`; keep the change small enough to review.
4. Run the validation commands listed below for every area you touched.
5. Archive with `/opsx-archive`, tick the roadmap item, and update affected docs in the same change.
6. Any decision that changes a crate boundary, a file format, the GPU data layout, the lighting
   model or a public interface needs an ADR in [docs/adr/](docs/adr/) (copy
   [0000-template.md](docs/adr/0000-template.md)).

## Repository layout (target; created in Phase 1)

```
crates/lightbaker/        upstream core: layout, BVH, raster, GPU passes, filters, output (platform-agnostic)
crates/lightbaker-cli/    upstream native CLI
crates/lightbaker-uv/     Unwrapper contract + native xatlas FFI (new; web uses xatlas-wasm in the worker)
crates/lightbaker-web/    wasm-bindgen bridge, browser GPU init, progress/cancel (new)
js/                       upstream TS package: scene writer / bake reader (extended, dependency-free)
app/                      React + Vite + Three.js editor (new)
test-scenes/              small deterministic scenes and their expected metrics (new)
docs/                     documentation and ADRs
openspec/                 OpenSpec specs and changes
```

Dependency direction is strict:
`app → js, lightbaker-web(pkg)`; `lightbaker-web → lightbaker, lightbaker-uv`;
`lightbaker-cli → lightbaker, lightbaker-uv`; `lightbaker` depends on no other crate of ours.
`lightbaker` and `js` must never depend on React, Three.js or anything browser-UI specific.

## Non-negotiable engineering rules

From the brief (§11) and the feasibility analysis:

1. **The bake runs in Rust/wgpu/WGSL.** Never implement a second lighting engine in JS/TS, and never
   replace wgpu with a JS renderer. Three.js only displays results and previews unbaked lighting.
2. **No silent CPU fallback.** If WebGPU is unavailable, the adapter is a software/fallback adapter,
   or the device is lost, the UI says so clearly. A software adapter (`isFallbackAdapter`, or
   llvmpipe/lavapipe, SwiftShader, WARP by name) may only be used when the user or CI explicitly
   opts in (`allowFallbackAdapter`, `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1`), and the result must be
   labelled. CI runs on such adapters by design (ADR-0009).
3. **No fake lighting.** Every light, emissive material and GI setting must participate in light
   transport. Artistic controls (AO strength, exposure) are allowed only where documented as such
   in [docs/LIGHTING_MODEL.md](docs/LIGHTING_MODEL.md).
4. **Never light twice.** Baked static meshes must not also receive the same real-time lights,
   environment light or AO in Three.js. See the rendering rules in
   [docs/LIGHTING_MODEL.md](docs/LIGHTING_MODEL.md#8-runtime-rendering-in-threejs).
5. **Preserve the upstream architecture.** Extend lightbaker's BVH, passes, tiles, cache and filters;
   don't rewrite them. Prefer additive modules and `cfg`/feature gates over edits to upstream
   logic, so upstream changes stay mergeable. Keep upstream's planar chart layout available.
6. **Native and WASM both build** (`lightbaker`, `lightbaker-uv`). Native-only code (rayon,
   `std::fs`, pollster, zstd-sys) sits behind the `native` feature; browser code behind `web`.
7. **Keep the engine independent of the UI and the UI independent of the GI algorithm.** The only
   contract between them is the interface set in [docs/INTERFACES.md](docs/INTERFACES.md).
8. **Correct minimal version first, then optimise.** Each feature lands with a test that proves it
   works before any performance work.

## WASM / WebGPU rules

- Never block: no `pollster::block_on`, `std::sync::mpsc::Receiver::recv`, `device.poll(wait…)`
  as a synchronisation primitive, `std::thread`, or busy loops in code compiled for `web`. GPU
  readback is `map_async` + an awaited future.
- `std::time::Instant`/`SystemTime` panic on `wasm32-unknown-unknown`: use the crate's time shim
  (`web-time` or equivalent, chosen in Phase 2).
- No `rayon` on the web target: use the crate's sequential/parallel iterator shim.
- No filesystem: the tile cache goes through the `TileStore` trait (memory/IndexedDB on the web).
- New dependencies must build for `wasm32-unknown-unknown` (or be gated to `native`). C/C++
  dependencies need an ADR describing how they build for the web.
- Respect WebGPU limits (`maxStorageBufferBindingSize`, `maxBufferSize`, workgroup limits). Request
  limits explicitly; split dispatches and buffers instead of assuming native limits.
- Keep large intermediate data on the GPU; every new GPU→CPU readback needs a justification in the
  change's design.
- Long dispatches trigger browser/driver watchdogs: keep per-submit work bounded (upstream batches).
- Baking runs in a **dedicated Web Worker**, never on the UI thread.
- Every `await` point in a bake loop checks the cancellation flag.

## GPU and shader rules

- Rust `#[repr(C)]` structs and their WGSL counterparts must match byte for byte; when you change
  one, change the other and the layout test in the same commit.
- Constants duplicated between Rust and WGSL (e.g. `PUSH`, `LIFT`) carry a comment naming the other
  location; keep them equal.
- Bump `BAKER_VERSION` (`crates/lightbaker/src/deps.rs`) whenever results for the same scene change,
  so stale cache tiles are not reused.
- Bakes are deterministic: the same scene and settings give the same bytes. Do not introduce
  time-, thread- or address-dependent seeds.
- WGSL must compile both with naga (native) and with the browser's compiler (Tint/Dawn, WGSL in
  Firefox/Safari). Avoid naga-only extensions.

## Lighting rules

- All lighting math is in **linear** RGB. Colour-space conversion happens only at asset import
  (sRGB colour → linear albedo) and at display.
- Diffuse transport is Lambertian and energy-conserving: `Lo = Le + (albedo / π) · E`.
  Albedo is clamped below 1 (see LIGHTING_MODEL.md).
- Units and falloff conventions match Three.js physically based lights so baked and unbaked views
  are comparable (see LIGHTING_MODEL.md).

## Code style

- Rust: edition 2024 (as upstream). Follow the surrounding upstream style; **do not reformat
  upstream files wholesale** (keeps diffs against upstream reviewable). New crates: `cargo fmt`
  and `cargo clippy -- -D warnings`.
- TypeScript: strict mode, ES modules, no `any` in public interfaces. `js/` stays dependency-free.
- Comment only what needs clarification (the *why*, units, invariants), not what the code says.
- Names of scene/bake JSON fields are `camelCase` (as upstream `serde(rename_all = "camelCase")`).

## Validation commands

The commands below are the **target** commands; they are created and verified in Phase 1/2 and
[docs/BUILD.md](docs/BUILD.md) is the authoritative, up-to-date list.

```powershell
cargo test -p lightbaker                       # unit + GPU integration tests (skip without a GPU locally)
cargo build -p lightbaker --target wasm32-unknown-unknown --no-default-features --features web
wasm-pack build crates/lightbaker-web --target web
pnpm -C js test
pnpm -C app test                               # Vitest
pnpm -C app e2e                                # Playwright: max. three specs (TESTING.md e2e budget)
```

Run the smallest set that covers your change. GPU tests that find no adapter are skipped locally,
not failed; say so in your summary when that happens. In CI they must run (lavapipe, ADR-0009).

**Testing restraint:** prove behaviour with Rust, WASM or Vitest tests. Do not add Playwright specs
beyond the budget in [docs/TESTING.md](docs/TESTING.md#e2e-budget); no image-diff tests in CI; no
network access in tests.

## Definition of done (per change)

- Behaviour proven by an automated test (unit, GPU integration, or deterministic scene metric).
- Native and WASM builds of touched crates pass.
- Docs and ADRs updated; roadmap item ticked; OpenSpec change archived.
- No new GPU→CPU readback, blocking call or native-only dependency on the web path without an ADR.
- Licences of any new dependency or asset recorded in `THIRD_PARTY_NOTICES.md` (created in Phase 1).

## Things not to do

- Don't build UI before the baker works in the browser (Phase 2 gate).
- Don't assume a polyfork.dev API exists beyond what [docs/ASSET_LIBRARY.md](docs/ASSET_LIBRARY.md)
  verified, and **never commit polyfork files** (link the CDN only).
- Don't integrate xatlas differently from [ADR-0006](docs/adr/0006-xatlas-integration.md)
  (native FFI over the pinned commit, `xatlas-wasm` in the worker) without a new ADR.
- Don't commit secrets, large binary assets without licence records, or generated `pkg/`/`dist/`.
- Don't hide failures: no swallowed GPU errors, no silently skipped tests in CI except "no GPU".
