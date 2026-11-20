# Requirements

Numbered requirements derived from [PROJECT_BRIEF.md](PROJECT_BRIEF.md). IDs are stable: reference
them from OpenSpec changes, tests and commits. **Phase** is where the requirement is first
satisfied (see [ROADMAP.md](../ROADMAP.md)). **Verify** names how it is proven:
U = unit test, G = GPU integration test (native), W = browser/WASM test, V = metric assertion on a
deterministic test scene (automated; image review is manual, see TESTING.md), M = manual check
recorded in the phase report.

## WASM / WebGPU engine (brief §2, §3)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-WASM-01 | The `lightbaker` crate compiles for `wasm32-unknown-unknown` with wasm-bindgen. | 2 | W (CI build) |
| REQ-WASM-02 | GPU initialisation is asynchronous and browser-compatible (no blocking calls). | 2 | W |
| REQ-WASM-03 | No native-only functionality (fs, threads, rayon, blocking) on the web path. | 2 | W, review |
| REQ-WASM-04 | Bakes execute on a real WebGPU adapter; the adapter is reported; software/fallback adapters are refused unless explicitly allowed (CI opts in, results labelled). | 2 | W, M |
| REQ-WASM-05 | Large intermediates stay on the GPU; readbacks are bounded and documented. | 2 → 6 | review, W (timing) |
| REQ-WASM-06 | Bakes are incremental (progress/preview per batch or tile) and cancellable within one batch. | 2 → 5 | W |
| REQ-WASM-07 | Device loss and adapter failures are caught and reported; the app can recreate the baker without reload. | 2 → 5 | W (forced `device.destroy()`) |
| REQ-WASM-08 | Capability check with a clear message when WebGPU baking is unavailable. | 2 → 5 | W, M |
| REQ-WASM-09 | Browser memory is budgeted: atlas/buffer sizes are checked against limits before allocation. | 2 → 6 | U, W |
| REQ-WASM-10 | Native build (CLI, tests) keeps working. | 1 → all | G |

## UV unwrapping (brief §2, §4)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-UV-01 | xatlas generates lightmap UVs for the Rust pipeline through its `Unwrapper` contract (native FFI; `xatlas-wasm` in the browser worker). | 3 | U, W |
| REQ-UV-02 | Configurable padding, texel density and atlas (page) resolution. | 3 | U |
| REQ-UV-03 | Original texture UVs are preserved; lightmap UVs go to a separate channel (`uv1`). | 3 | U, W |
| REQ-UV-04 | Meshes with multiple material groups are supported. | 3 | U |
| REQ-UV-05 | Multiple lightmap atlases (pages) are supported. | 3 | U, G |
| REQ-UV-06 | Chart fragmentation is controlled (chart count / utilisation metrics within thresholds). | 3 | U |
| REQ-UV-07 | Charts and atlas packing are visualised in the app. | 5 | M |
| REQ-UV-08 | The upstream planar chart layout remains selectable. | 3 | G |
| REQ-UV-09 | Generated UVs are valid: no overlaps between charts, padding respected, no degenerate/flipped triangles. | 3 | U |

## Lighting (brief §5)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-LGT-01 | Directional light / sun with angular radius (soft shadows). | 1 (upstream) | G |
| REQ-LGT-02 | Point lights: position, RGB colour, intensity, range/inverse-square falloff. | 4 | G, V |
| REQ-LGT-03 | Spotlights: position, direction, inner/outer cone, colour, intensity. | 4 | G, V |
| REQ-LGT-04 | Area lights (rect, disk) with stochastic sampling (if feasible). | 4 | G, V |
| REQ-LGT-05 | Sky / environment illumination. | 1 (upstream) | G |
| REQ-LGT-06 | Emissive materials (colour, intensity) contribute to indirect lighting. | 4 | G, V |
| REQ-LGT-07 | Materials: albedo, emission, roughness (display), opacity. | 4 | U |
| REQ-LGT-08 | Lambertian transport `Lo = Le + (albedo/π)·E`. | 4 | G (furnace) |
| REQ-LGT-09 | Colour bleeding from emissive surfaces onto nearby geometry. | 4 | V |
| REQ-LGT-10 | Per-vertex/per-triangle albedo (vertex colours) participates in bounces (needed for vertex-coloured polyfork assets). | 4 | G, V |
| REQ-LGT-11 | Double-sided materials (foliage cards) are handled in bounces and inside-solid detection. | 4 | G |

## Global illumination quality (brief §6)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-GI-01 | Configurable multiple-bounce diffuse GI. | 1 (upstream) → 4 | G |
| REQ-GI-02 | Energy conserving; no energy explosion with many bounces. | 4 | G (furnace) |
| REQ-GI-03 | Visible colour bleeding (Test 1). | 4 | V |
| REQ-GI-04 | Soft shadows: sun angular radius, area lights; filtering/denoising. | 4 → 6 | V |
| REQ-GI-05 | Ambient occlusion with configurable intensity and distance; no excessive darkening. | 1 (upstream) → 4 | G |
| REQ-GI-06 | Interpolation (irradiance-cache-inspired) with configurable quality and no leaks across geometry boundaries. | 6 | G, V |
| REQ-GI-07 | Quality presets Preview / Balanced / High Quality; all parameters overridable. | 5 | U, M |

## Browser application (brief §7)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-APP-01 | Three.js viewport with orbit controls, selection, transform gizmos. | 5 | W (e2e) |
| REQ-APP-02 | Scene hierarchy and material editing. | 5 | W (e2e) |
| REQ-APP-03 | Lighting visualisation (gizmos) and editor for directional/point/spot lights and emissive materials. | 5 | W (e2e) |
| REQ-APP-04 | Toggle baked / unbaked lighting; toggle lightmap-only display; UV chart view. | 5 | W, M |
| REQ-APP-05 | Bake controls: atlas resolution, texel density, bounces, sampling, shadow quality, AO, denoising, incremental. | 5 | W |
| REQ-APP-06 | Progress, elapsed time and ETA; cancel. | 5 | W |
| REQ-APP-07 | Modify lighting and rebake without reloading. | 5 | W (e2e) |
| REQ-APP-08 | Feels like a lightweight editor, not a shader demo (undo/redo, save/load scene). | 5 | M |

## Assets (brief §7)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-AST-01 | polyfork.dev delivery, API and licence verified before integration (documented). | 0 → 5 | M ([ASSET_LIBRARY.md](ASSET_LIBRARY.md)) |
| REQ-AST-02 | GLTF/GLB importer preserving geometry, materials, multiple meshes and material groups. | 5 | U, W |
| REQ-AST-03 | Asset browser to add assets to the scene (polyfork.dev if permitted, else curated local library). | 5 | W |
| REQ-AST-04 | Primitive geometry as an alternative. | 5 | W |
| REQ-AST-05 | Licence and attribution recorded for every bundled asset. | 5 | review |

## Demo and visual target (brief §8)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-DEMO-01 | Test 1: colour bleeding scene. | 4 (test) / 6 (showcase) | V |
| REQ-DEMO-02 | Test 2: emissive lighting in a dark environment. | 4 / 6 | V |
| REQ-DEMO-03 | Test 3: coloured point lights in architecture. | 4 / 6 | V |
| REQ-DEMO-04 | Test 4: bounce-count progression in an enclosed coloured room. | 4 / 6 | V |
| REQ-DEMO-05 | Test 5: stylized outdoor scene (sun, sky, trees, rocks, architecture). | 6 | V, M |
| REQ-DEMO-06 | Showcase environment reminiscent of The Witness. | 6 | M (review against references) |

## Export and runtime (brief §9)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-OUT-01 | HDR lightmap output (RGBA16F / RGB9E5). | 3 → 5 | U |
| REQ-OUT-02 | Linear colour handling end to end. | 5 | W (calibration test) |
| REQ-OUT-03 | Correct UV assignment, filtering, padding/dilation, mipmap-safe handling. | 3 → 6 | U, V |
| REQ-OUT-04 | Baked-lighting material configuration in Three.js without double lighting. | 5 | W (calibration test) |
| REQ-OUT-05 | Export: GLB-compatible scene + lightmaps, documented. | 5 | U (round trip) |

## Engineering (brief §10, §11)

| ID | Requirement | Phase | Verify |
|---|---|---|---|
| REQ-ENG-01 | No JS lighting engine; no silent CPU fallback; no fake lighting. | all | review |
| REQ-ENG-02 | Engine independent of React; UI independent of the GI algorithm; typed interfaces. | all | review, build graph |
| REQ-ENG-03 | Reproducible build instructions (native + WASM + app). | 1 → 2 | CI |
| REQ-ENG-04 | Automated tests for UV generation, lighting calculations and bake output. | 3 → 6 | CI |
| REQ-ENG-05 | Small deterministic test scenes for visual regression. | 2 → 6 | CI |
| REQ-ENG-06 | Major architectural changes documented (ADRs). | all | review |
| REQ-ENG-07 | Upstream architecture preserved; upstream changes remain mergeable. | all | review |

## Definition of success (brief §12)

| # | Success criterion | Requirements | Phase |
|---|---|---|---|
| S1 | Engine compiles to WASM | REQ-WASM-01 | 2 |
| S2 | WebGPU used for real GPU baking in a browser | REQ-WASM-02, -04 | 2 |
| S3 | xatlas generates valid lightmap UVs | REQ-UV-01, -09 | 3 |
| S4 | Scene populated with imported low-poly assets | REQ-AST-02, -03 | 5 |
| S5 | Point lights and emissive materials contribute to baked GI | REQ-LGT-02, -06 | 4 |
| S6 | Multiple-bounce indirect lighting works | REQ-GI-01, -02 | 4 |
| S7 | Colour bleeding clearly visible | REQ-GI-03, REQ-DEMO-01 | 4 |
| S8 | Baked lightmaps render correctly in Three.js | REQ-OUT-02, -04 | 5 |
| S9 | Modify lighting and rebake without reload | REQ-APP-07 | 5 |
| S10 | Showcase reminiscent of The Witness | REQ-DEMO-06 | 6 |
