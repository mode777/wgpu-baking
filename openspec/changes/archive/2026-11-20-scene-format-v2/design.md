# Design: scene-format-v2

## Context

`Scene { header, positions: Vec<Vec<Vec3>>, hash }`; `MeshInfo { name, material, triangles }`; the engine
reads mesh materials in `bvh.rs`, `deps.rs`, `layout.rs`, `bake.rs`. The bridge rewrites a scene after config
overrides with `Scene::write(header, positions)` (v1 only). INTERFACES.md §5 sketches v2.

## Goals / Non-Goals

**Goals:** v2 on disk in both languages; engine unchanged for v1 and indifferent to v2's extra data until the
features that use it land. **Non-Goals:** compression, multiple UV sets, skinning.

## Decisions

- **Split by material at read time.** A v2 mesh with `materialIds` becomes one engine mesh per distinct id (in
  id order), each with `origin { file_mesh, triangles }`. Upstream code keeps one material per mesh and needs no
  change. Alternative: per-triangle material everywhere (BVH, layout, deps) — six call sites in upstream files,
  for no behaviour gain now. The split is invisible in outputs because results are mapped back through
  `origin` (P3.8).
- **Expanded per-corner attributes** (`normals`, `colors`, `lightmap_uvs`: `Vec<Option<Vec<…>>>` parallel to
  `positions`) — what raster and BVH will consume in P3.3–P4.3; the file's indexed form is kept in
  `Scene::file` (`SceneFile { header, meshes: Vec<FileMesh> }`) for rewriting and for per-vertex outputs.
- **Header compatibility:** `MeshInfo` gains `vertices: Option<u32>` and `arrays: Option<Vec<String>>`
  (absent in v1); `Material` gains `emission` (default `[0,0,0]`) and `double_sided` (default false, JSON
  `doubleSided`); `SceneHeader` gains `lights: Vec<serde_json::Value>` (default empty; typed in P4.1). Defaults
  are skipped when serialising, so v1 headers write unchanged bytes.
- **Array grammar:** `name:type` with types `f32x3`, `f32x2`, `u32`, `u16`; lengths: per vertex for
  positions/normals/colors/lightmapUv, `3 × triangles` for indices, `triangles` for materialIds; `triangles`
  in `MeshInfo` is required and must agree (`indices/3`, or `vertices/3` without indices). Unknown array names
  are errors (no silent loss).
- **Refusal of unsupported inputs** in `bake()` (not in the reader), so files can carry them and tools can
  read them.
- **Golden files** regenerated with `LIGHTBAKER_UPDATE_GOLDEN=1` (Rust) / `UPDATE_GOLDEN=1` (TS); otherwise
  tests compare against them, pinning both writers.
- **ADR-0010** records the format decision (a format change needs an ADR, AGENTS.md).

## Risks / Trade-offs

- [Split meshes change mesh indices in logs/stats] → names keep the file mesh name; `origin` maps back.
- [JSON float formatting differs between writers] → goldens are compared structurally across languages,
  byte-for-byte only per writer.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none. ADRs: ADR-0010 (new).
