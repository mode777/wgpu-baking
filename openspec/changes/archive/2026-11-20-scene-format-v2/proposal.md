# Proposal: scene-format-v2 (P3.2)

## Why

The `.lbscene` v1 format carries only non-indexed positions and one material per mesh. Phase 3 needs indexed
meshes with normals and lightmap UVs (xatlas, per-triangle UVs, UV-space rasterisation), Phase 4 needs lights,
emission, double-sided materials and vertex colours (polyfork assets are vertex-coloured), and the editor
(Phase 5) exports exactly this (INTERFACES.md §5). Rust and TypeScript must read and write the same format.
Requirements: REQ-UV-02, REQ-UV-08, REQ-LGT-10 (data path), REQ-OUT-03 (partial), REQ-ENG-05.

## What Changes

- `.lbscene` **v2** (version 2, same framing): per mesh `vertices` and an `arrays` list
  (`positions:f32x3` required, optional `normals:f32x3`, `indices:u32`, `materialIds:u16`, `colors:f32x3`,
  `lightmapUv:f32x2`), each array 4-byte aligned, in mesh then list order. Header gains `lights` (default
  empty); materials gain `emission` (default black) and `doubleSided` (default false).
- Rust: `Scene::read` reads v1 and v2. The engine's view stays upstream's (non-indexed triangles per mesh,
  one material per mesh): a v2 mesh with several material ids becomes one engine mesh per material, each
  remembering its file mesh and triangles; per-vertex arrays are also expanded per triangle corner.
  `Scene::write_v2` writes v2; `Scene::to_bytes` rewrites a scene in its own version (used by the bridge's
  config overrides).
- A bake fails with a clear "not supported yet" error for non-empty `lights` (until P4.1), non-zero
  `emission` (P4.2) and `doubleSided` (P4.3), never silently ignoring them.
- TypeScript (`js/src/scene.ts`): v2 types, `writeSceneV2`, `readScene` for v1 and v2.
- Golden files `test-scenes/format-v2/{rust,ts}.lbscene`: each side reads the other's; writers are pinned;
  `upstream-basic` converted to indexed v2 bakes to the same light as v1.

## Capabilities

### New Capabilities

- `scene-format`: the `.lbscene` file format (v1 and v2), its validation, and how both languages read and write it.

### Modified Capabilities

None.

## Out of scope

Using normals (P3.4), lightmap UVs (P3.3/P3.6), lights (P4.1), emission (P4.2), vertex albedo and double-sided
(P4.3) in the bake; `.lbake` v2 (P3.8).

## Impact

`crates/lightbaker/src/scene.rs` (+ engine-side checks in `bake.rs`), `crates/lightbaker-web` (rewrite via
`to_bytes`), `js/src/scene.ts`, tests, `test-scenes/format-v2/`, docs/FORMAT.md, INTERFACES.md §5, ADR-0010
(file format v2).
