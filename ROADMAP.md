# Roadmap

Incremental plan from the brief's six phases ([PROJECT_BRIEF.md §10](docs/PROJECT_BRIEF.md)), refined
by the [feasibility report](docs/FEASIBILITY.md). Each task is one **OpenSpec change** (suggested
name in `code`), small enough to review. Requirement IDs refer to
[docs/REQUIREMENTS.md](docs/REQUIREMENTS.md); S1–S10 are the brief's success criteria.

**Rules**

- Work only on the current phase. A phase ends when **all** its exit criteria are met and its
  report is committed in `docs/reports/`.
- **Hard gate:** nothing from Phase 3 onward starts before the Phase 2 exit criteria (the existing
  baker baking in a browser on a real WebGPU adapter) are met.
- Tick a task (`[x]`) in the same change that archives its OpenSpec change.
- Performance targets are set at the end of Phase 2 from measurements, not guessed now.

**Current phase: 2 — WASM / WebGPU proof of concept** (Phase 1 complete: [report](docs/reports/phase-1-baseline.md)).

---

## Phase 0 — Research and planning ✅

- [x] Inspect upstream lightbaker (commit `4fe85da`) — [FEASIBILITY.md](docs/FEASIBILITY.md)
- [x] Research The Witness articles, xatlas, wgpu/WebGPU, polyfork.dev — [RESEARCH.md](docs/RESEARCH.md), [ASSET_LIBRARY.md](docs/ASSET_LIBRARY.md)
- [x] Requirements, architecture, lighting model, interfaces, testing, demo scenes, build plan
- [x] ADRs 0001–0009 (proposed/accepted)
- [x] Open questions Q1, Q2, Q5, Q6 resolved with the project owner (see below)
- [x] AGENTS.md and this roadmap

---

## Phase 1 — Understand and fork ✅

**Goal:** a clean fork with reproducible native builds and a measured baseline.
**Deliverable:** documented technical architecture (verified) and a working native baseline.

- [x] **P1.1** `fork-baseline` — Initialise git here, add `upstream` remote (jer-nc/lightbaker),
      merge upstream history (`--allow-unrelated-histories`, since this folder already holds docs
      and `openspec/`), move upstream README to `docs/UPSTREAM_README.md` (ADR-0001).
- [x] **P1.2** `repo-hygiene` — `rust-toolchain.toml`, `.gitignore` additions (`pkg/`, `dist/`,
      `node_modules/`), `THIRD_PARTY_NOTICES.md`; `LICENSE`: keep upstream's MIT text and
      `Copyright (c) 2026 Orbitoon`, add `Copyright (c) 2026 The wgpu-baking contributors`.
- [x] **P1.3** `ci-native` — GitHub Actions (ADR-0009): `rust-native` (build, unit tests, clippy
      for new crates; Ubuntu + Windows) and `rust-gpu` (Ubuntu + `mesa-vulkan-drivers`/lavapipe,
      upstream GPU test with `LIGHTBAKER_REQUIRE_GPU=1`; add the env checks to the test helper).
- [x] **P1.4** `upstream-test-scene` — Export upstream's `tests/bake.rs` scene to
      `test-scenes/upstream-basic/` (`.lbscene` + `expect.json` with its metrics) via a small example
      binary; CLI bakes it.
- [x] **P1.5** `phase-1-baseline` — Run upstream tests on a real GPU (not skipped), record adapter,
      timings per stage, readback counts, memory; verify FEASIBILITY.md against the running code.
      Report: `docs/reports/phase-1-baseline.md`.

**Exit criteria**

1. `cargo test -p lightbaker` passes natively with a GPU, not skipped: in CI on lavapipe and
   locally on a hardware GPU (recorded in the report).
2. `lightbaker-cli` bakes `test-scenes/upstream-basic` reproducibly (identical bytes twice).
3. Baseline report committed; FEASIBILITY.md corrected if anything differed.

Requirements: REQ-WASM-10, REQ-ENG-03, REQ-ENG-07, REQ-LGT-01, REQ-LGT-05, REQ-GI-01 (upstream).

---

## Phase 2 — WASM / WebGPU proof of concept

**Goal:** the *existing* baker (planar layout, sun, sky, bounces, volume) bakes a small scene in
the browser on WebGPU. **Deliverable:** browser PoC page baking `upstream-basic`.

- [x] **P2.1** `platform-shims` — Features `native`/`web`; `web-time`; `par` shim; `TileStore`
      (`FsStore`/`MemoryStore`); zstd gated (ADR-0003, blockers B4–B7). Native tests unchanged.
- [x] **P2.2** `async-gpu` — `Gpu::open` async, async readback, `bake()` async; CLI/tests via
      `block_on`; explicit limits; adapter report (B1–B3, B8).
- [x] **P2.3** `ci-wasm` — CI `wasm` job: `lightbaker` for `wasm32-unknown-unknown --features web`,
      later `wasm-pack build` and `wasm-bindgen-test` (Node).
- [x] **P2.4** `wasm-bridge` — `crates/lightbaker-web`: `capabilities()`, `WebBaker::create/bake/cancel`,
      progress callback, typed errors, device-lost callback (INTERFACES.md §7–8).
- [ ] **P2.5** `browser-poc` — Minimal `app/` (Vite, no React editor yet): baker worker (ADR-0002),
      loads `upstream-basic.lbscene`, bakes, shows adapter info, progress, timings and the
      direct/indirect textures on a simple Three.js view.
- [ ] **P2.6** `browser-ci` — CI `web` job (lavapipe + xvfb, Chrome WebGPU flags) with the first
      two e2e specs of the budget (TESTING.md): `bake-poc.spec.ts` (CI-sized `upstream-basic`,
      metrics vs `expect.json`, cancel + rebake) and `no-webgpu.spec.ts`. Software adapter allowed
      and labelled.
- [ ] **P2.7** `cancel-and-device-loss` — Cancel within one batch (Rust test); device-lost mapping
      and worker recreation (Vitest with mocked `GPUDevice.lost`); real `device.destroy()` checked
      manually on a hardware GPU.
- [ ] **P2.8** `phase-2-report` — Native vs browser timings, readbacks, limits hit, WGSL compile
      notes, memory; **set performance targets** for Phases 3–6. Report: `docs/reports/phase-2-webgpu.md`.

**Exit criteria (hard gate)**

1. S1: `lightbaker` compiles to `wasm32-unknown-unknown`; CI enforces it.
2. S2: the upstream test scene bakes in Chrome/Edge on a **hardware** WebGPU adapter (manual,
   recorded in the report) and on lavapipe in CI; metrics match native; adapters recorded.
3. Cancel and device-loss behaviours tested; native build and tests still green.

Requirements: REQ-WASM-01 … 04, -06, -07, -08 (basic), -10; REQ-ENG-05 (first scene).

---

## Phase 3 — xatlas lightmap UVs

**Goal:** automatic lightmap UVs with xatlas feeding the unchanged GI pipeline.
**Deliverable:** browser bake of imported-style meshes with xatlas UVs, no leaks.

- [ ] **P3.1** `xatlas-wasm-evaluation` — ≤ 1 day: load pinned `xatlas-wasm` in the baker worker
      under Vite, unwrap test meshes per object tile, confirm options/outputs needed by ADR-0004,
      record timings and memory in ADR-0006.
- [ ] **P3.2** `scene-format-v2` — `.lbscene` v2 in Rust and TS (indexed meshes, normals, material
      ids, colours, lightmap UVs); v1 still read; golden-file tests.
- [ ] **P3.3** `per-triangle-uvs` — `Tri` 80 B with UVs/page; `trace()` barycentrics; `bounced()`
      via UVs; `charts` binding removed; planar layout emits per-triangle UVs; Rust↔WGSL layout
      tests; metrics of `upstream-basic` unchanged (ADR-0004). Bump `BAKER_VERSION`.
- [ ] **P3.4** `uv-space-raster` — Rasterise from UVs; interpolated vertex normals.
- [ ] **P3.5** `xatlas-native` — `crates/lightbaker-uv` with vendored `jpcy/xatlas@f700c77` + FFI;
      `Unwrapper` trait; UV validity test suite (TESTING.md).
- [ ] **P3.6** `xatlas-web` — JS adapter over `xatlas-wasm` imported by `lightbaker-web`; UV validity
      suite in the browser path; native/web equivalence test.
- [ ] **P3.7** `object-tiles-pages` — Object tiles packed by xatlas, shelved by upstream
      `shelve()`; multiple pages; tile reuse after moving one object.
- [ ] **P3.8** `lightmap-uv-output` — Per-vertex `uv1` + index remap in results and `.lbake`;
      PoC page displays an xatlas-baked mesh with `uv1` lightmap.
- [ ] **P3.9** `leak-and-padding` — `leak`, `uv-multi-material`, `uv-multi-page` scenes; padding
      dilation checks.

**Exit criteria**

1. S3: xatlas UVs valid (no overlaps, padding, no flips) natively and in the browser.
2. Planar layout still selectable and unchanged in results.
3. Moving one object rebakes only nearby tiles; multi-page scenes bake.

Requirements: REQ-UV-01 … 06, -08, -09; REQ-OUT-01, -03 (partial).

---

## Phase 4 — Lighting extensions

**Goal:** a general-purpose lightmapper. **Deliverable:** deterministic Tests 1–4 passing natively
and in the browser.

- [ ] **P4.1** `point-lights` — Lights in scene v2 (uniform buffer), `Lights` pass, point lights
      with Three.js-compatible falloff; `point-falloff` test.
- [ ] **P4.2** `emissive-materials` — Emission in materials and `bounced()`; `furnace` and
      `emissive-bleed` tests.
- [ ] **P4.3** `vertex-albedo-double-sided` — Per-triangle albedo from vertex colours;
      `doubleSided` in traversal and inside test; `thin-double-sided` test.
- [ ] **P4.4** `spot-lights` — Inner/outer cone; `spot-cone` test.
- [ ] **P4.5** `area-lights` — Rect and disk, stratified sampling, `sourceRadius` for point/spot;
      `area-rect` test.
- [ ] **P4.6** `combined-output` — Combined HDR irradiance output, AO on indirect only, albedo
      clamp; lights in tile hashes.
- [ ] **P4.7** `gi-test-scenes` — Deterministic Tests 1–4 (`color-bleed`, `emissive-bleed`,
      point-light arcade, `bounces`) with metrics, run natively and in the browser.

**Exit criteria**

1. S5: point lights and emissive materials measurably contribute to direct **and** indirect light.
2. S6: bounce progression converges; furnace test within 2 %.
3. S7: colour-bleeding metrics pass.

Requirements: REQ-LGT-02 … 04, -06 … -11; REQ-GI-01 … 03, -05; REQ-DEMO-01 … 04 (tests).

---

## Phase 5 — Demo application

**Goal:** a functional editor. **Deliverable:** build a scene from assets, light it, bake, compare,
edit, rebake — without reload.

- [ ] **P5.0** `phase-5-prep` — Re-verify polyfork terms (licensing, hotlinking, API); confirm
      ADR-0007/0008.
- [ ] **P5.1** `editor-shell` — React app shell, Zustand `SceneDocument`, commands, undo/redo,
      autosave, scene JSON import/export.
- [ ] **P5.2** `editor-viewport` — Imperative viewport, orbit, selection, transform gizmos,
      hierarchy panel.
- [ ] **P5.3** `lightmap-calibration` — Three.js calibration (LIGHTING_MODEL.md §8): automated for
      the WebGPU backend (step of `editor-smoke.spec.ts`), manual for WebGL; pin the Three.js
      version.
- [ ] **P5.4** `editor-lights-materials` — Light creation/editing with helpers; material editor
      (albedo, emission, opacity, double-sided).
- [ ] **P5.5** `editor-bake-panel` — Presets + advanced parameters, progress/elapsed/ETA, cancel,
      progressive preview, apply results; views: unbaked / baked / lightmap only / UV charts /
      validity.
- [ ] **P5.6** `asset-import` — GLB import (materials, groups, vertex colours), primitives library.
- [ ] **P5.7** `polyfork-browser` — Search, preview, insert from the CDN (never cached or committed),
      credits panel, back-off (ADR-0008); tested with a mocked API, no network in CI.
- [ ] **P5.8** `export-bundle` — GLB + lightmaps export, Three.js loader helper,
      `docs/EXPORT_FORMAT.md`; round-trip test (Vitest, Node).
- [ ] **P5.9** `editor-e2e` — The third and last e2e spec, `editor-smoke.spec.ts`: primitive +
      point light → bake (CI profile) → toggle → move light → rebake, plus the calibration step.

**Exit criteria**

1. S4: a scene populated with imported low-poly assets bakes.
2. S8: calibration test passes; baked lightmaps render correctly without double lighting.
3. S9: lighting edits and rebakes work without reload (e2e).

Requirements: REQ-APP-01 … 08, REQ-AST-01 … 05, REQ-UV-07, REQ-GI-07, REQ-OUT-01 … 05,
REQ-WASM-06 … 08 (full).

---

## Phase 6 — Visual quality and optimisation

**Goal:** Witness-like quality at interactive bake times. **Deliverable:** polished showcase.

- [ ] **P6.1** `gpu-resident-light` — Light atlas and gather results stay on the GPU; fills/smooth
      as compute; readbacks only for final pages (REQ-WASM-05).
- [ ] **P6.2** `adaptive-interpolation` — Irradiance-cache-inspired refinement and interpolation
      (LIGHTING_MODEL.md §5, ADR-0005); `leak` scene passes in all modes.
- [ ] **P6.3** `denoise-seams-mips` — GPU à-trous denoiser, seam stitching, mip-safe dilation.
- [ ] **P6.4** `emissive-nee` — Optional, only if emissive noise remains visible at Balanced.
- [ ] **P6.5** `robustness` — Adaptive batch sizes (watchdog), memory budget, limit errors,
      device-loss recovery under load.
- [ ] **P6.6** `showcase-scene` — Test 5 outdoor scene and the showcase plaza with vignettes,
      camera bookmarks (DEMO_SCENES.md).
- [ ] **P6.7** `preset-tuning` — Tune Preview/Balanced/High against Phase 2 targets.
- [ ] **P6.8** `browser-matrix` — Browser × GPU matrix report on real hardware (manual, TESTING.md);
      optional `gpu-perf` workflow for timings (ADR-0009).
- [ ] **P6.9** `final-report` — Requirements traceability (all REQ-… verified), known limits.

**Exit criteria**

1. S10: showcase reviewed against the art direction in DEMO_SCENES.md.
2. Performance targets from Phase 2 met for Balanced on the reference GPUs.
3. All requirements verified; final report committed.

---

## Milestones

| Milestone | Phase end | Success criteria |
|---|---|---|
| M1 Native baseline | 1 | — (foundation) |
| M2 Baker in the browser | 2 | S1, S2 |
| M3 Automatic lightmap UVs | 3 | S3 |
| M4 General lightmapper | 4 | S5, S6, S7 |
| M5 Interactive editor | 5 | S4, S8, S9 |
| M6 Showcase | 6 | S10 |

## Open questions

| # | Question | Needed by | Owner |
|---|---|---|---|
| Q3 | Three.js `WebGPURenderer` vs `WebGLRenderer` for viewing (lightmap semantics) | P5.3 | calibration test |
| Q4 | Concrete performance targets (bake time per preset, max scene size) | P2.8 | measurements |

### Resolved (2026-11-20)

| # | Question | Decision |
|---|---|---|
| Q1 | xatlas in the browser | `xatlas-wasm` in the baker worker; native FFI over the same xatlas commit (ADR-0006). |
| Q2 | polyfork files in the repo; browser identification | Never commit polyfork meshes (script-generated; committing could count as redistribution); link the CDN only. Browser `Referer`/`Origin` is acceptable instead of a custom `User-Agent` (ADR-0008). |
| Q5 | Copyright line | Keep upstream's MIT text and Orbitoon line; add `Copyright (c) 2026 The wgpu-baking contributors` (P1.2). |
| Q6 | CI with GPU/WebGPU | GitHub Actions standard runners with software adapters (lavapipe), small e2e budget; real-GPU checks manual or optional paid runner (ADR-0009). |

## Risks

See [FEASIBILITY.md §9](docs/FEASIBILITY.md#9-risks-and-mitigations). Re-assess at every phase end.
