# ADR-0004: Per-triangle lightmap UVs and object tiles

- **Status:** proposed (implemented in Phase 3)
- **Date:** 2026-11-20
- **Phase:** 3
- **Requirements:** REQ-UV-01 … 09, REQ-WASM-06

## Context

Upstream represents lightmap UVs implicitly: each **chart** is planar and has an **affine map**
from world position to atlas texel (`Chart::affine`, `layout.rs`). That map is used in three places:

1. `raster.rs` rasterises triangles through `Chart::local` (world → chart texel);
2. `trace.wgsl::bounced()` finds the lit texel of a ray hit with `charts[c*2..c*2+1]` applied to
   the hit position (`Tri.e1.w` = chart index);
3. `js/src/uv.ts` recomputes UVs on the engine side from positions and the chart map.

xatlas produces **non-planar charts with per-vertex UVs** (vertices split along seams, `xref` back
to the input vertex). No affine map exists for such charts.

Upstream's incremental baking relies on **tiles**: each tile packs its own charts into its own
rectangle, tiles are shelved into the atlas, and a tile's light is cached by the hash of what can
reach it (`deps.rs`).

## Decision

1. **Per-triangle UVs on the GPU.** A new storage buffer `tri_uv: array<vec4f>` (two entries per
   BVH triangle: `(u0,v0,u1,v1)`, `(u2,v2,page,_)` in atlas texel coordinates) replaces the
   `charts` buffer. `trace()` returns the hit's barycentrics; `bounced()` interpolates the UV and
   reads `light[page][…]`. The planar layout also emits per-triangle UVs (evaluating its affine
   map at the vertices), so **both unwrappers share one downstream path**.
2. **Rasterisation in UV space.** `raster.rs` rasterises each triangle from its tile-local UVs
   (same conservative two-pass algorithm). Position, `dx`, `dy` come from the UV→world Jacobian;
   normals are interpolated when the mesh has vertex normals, else the face normal (upstream).
3. **Object tiles.** For xatlas, a *tile* is an object (or a group of small nearby objects): xatlas
   computes and packs that tile's charts into its own rectangle (texel density from
   `texelsPerMeter`, padding from settings), and upstream's `shelve()` places tile rectangles in
   the atlas — exactly upstream's two-level scheme. Tile hashes (`deps.rs`) stay valid, so moving
   one object rebakes only the tiles within reach. Objects too large for one page are split
   spatially with upstream's ground-plane tiles.
4. **Pages.** `shelve()` opens a new page when a page reaches `atlasMaxSize`. The `light` atlas
   becomes one buffer per page (or one buffer with page offsets, subject to binding-size limits).
5. **Output.** Lightmap UVs ship per vertex (`uv1`) with the xatlas index remap; upstream
   `charts`/`triangleCharts` chunks are still written in `planar` mode for `.lbake` compatibility.

## Alternatives considered

| Option | Why not |
|---|---|
| One global xatlas pack for the whole scene | Best packing, but any change moves charts → no tile reuse; incremental rebakes lost. Kept as an option (`tiles: 'global'`) for final exports. |
| Keep affine charts, use xatlas only for chart segmentation and force planar projection | Defeats xatlas' purpose (distortion-free non-planar charts on curved assets). |
| Store a UV texture/"position map" instead of per-triangle UVs | More memory, more sampling error; per-triangle UVs are exact and small (32 B/triangle). |

## Consequences

- `Tri`/`Hit` structs, `Pass` bindings, `raster.rs`, `bake.rs` (affines), `output.rs`, `js/uv.ts`
  change; Rust↔WGSL layout tests are mandatory.
- Slightly worse packing than a global pack (rectangles per tile); mitigated by grouping small
  objects and by the optional global mode.
- Bump `BAKER_VERSION`.

## Verification

Phase 3: upstream test scene baked with planar-via-per-triangle-UV gives the same metrics as
before (± noise); xatlas test meshes pass the UV validity tests; moving one object rebakes only
nearby tiles.
