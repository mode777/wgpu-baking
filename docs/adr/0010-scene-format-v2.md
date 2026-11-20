# ADR-0010: Scene file format version 2

- **Status:** accepted
- **Date:** 2026-11-20
- **Phase:** 3
- **Requirements:** REQ-UV-02, REQ-UV-08, REQ-LGT-10, REQ-OUT-03, REQ-ENG-05

## Context

Upstream's `.lbscene` v1 stores non-indexed positions with one material per mesh. xatlas works on indexed
meshes with normals (ADR-0006), UV-space rasterisation wants vertex normals (ADR-0004), polyfork assets carry
vertex colours and per-triangle materials (ASSET_LIBRARY.md), and Phase 4 adds lights, emission and
double-sided materials. INTERFACES.md §5 sketched the additions; Rust and TypeScript must agree byte for byte
on the framing.

## Decision

- **Version 2, same framing** (`LBSC`, version, header length, JSON header, 4-byte aligned arrays). Each mesh
  declares `vertices` and an ordered `arrays` list of `name:type` entries: `positions:f32x3` (required),
  `normals:f32x3`, `indices:u32`, `materialIds:u16`, `colors:f32x3`, `lightmapUv:f32x2`. Unknown names are errors.
- Header additions with defaults that are **not serialised** (`lights: []`, `emission: [0,0,0]`,
  `doubleSided: false`), so v1 headers and the v1 writer are unchanged.
- **Engine view unchanged:** readers expand v2 meshes to upstream's non-indexed triangles and split a mesh
  with several material ids into one engine mesh per material; `Scene::origin` maps engine meshes back to
  file meshes and triangles; per-vertex arrays are also available per triangle corner. Upstream modules
  (layout, BVH, deps, raster) need no change for v2.
- **Inputs the engine cannot bake yet are refused** by `bake()` with a named error (lights, emission,
  double-sided), never ignored (AGENTS.md rule 3).
- v1 stays readable and writable; `Scene::to_bytes` rewrites a scene in its own version.
- Golden files per writer, cross-read by the other language.

## Alternatives considered

| Option | Pros | Cons | Why not |
|---|---|---|---|
| glTF/GLB as the bake input | standard, tooling | large parser in the engine (wasm size), materials/lights semantics differ; still needs our settings | the editor converts GLB to `.lbscene` (Phase 5) |
| Per-triangle material ids throughout the engine | no mesh split | six call sites in upstream files change | the split keeps upstream untouched; `origin` maps results back |
| Fixed array layout (all arrays always present) | simpler reader | wasteful, ambiguous "absent" | named arrays are explicit and extensible |

## Consequences

- Later features read `Scene::{normals, colors, lightmap_uvs}` instead of the file.
- Engine mesh indices differ from file mesh indices for multi-material meshes; outputs per file vertex (P3.8)
  must go through `origin`.
- New array kinds need an ADR update and both readers.

## Verification

`scene::tests` (round trip, errors, split), `tests/format_v2.rs` and `js/test/scene.test.ts` (golden files
both ways), `tests/scenes.rs::indexed_v2_bakes_like_v1` (same light for the same triangles); `upstream-basic`
v1 `.lbake` bytes unchanged.
