# Design: uv-space-raster

## Context

`rasterize()` loops charts → triangles, computes `c.local(p)` per corner (tile-local texel coordinates), runs a
two-pass (centre, then conservative) scan with barycentrics in UV space, and writes `Texel { pos, normal=face,
dx, dy }`. `Scene::normals` (P3.2) holds per-corner normals per engine mesh.

## Goals / Non-Goals

**Goals:** one UV source for raster; smooth normals for v2; identical v1 output. **Non-Goals:** changing the
conservative coverage rules.

## Decisions

- **`Layout::triangle_uvs(scene, chart, tri) -> [Vec2; 3]`** (tile-local texels): the single point where the
  planar layout's projection is applied; P3.6 makes xatlas charts answer it from their stored UVs.
- **Normal per texel** from the same barycentrics as the position; fall back to the face normal when
  `dot(n, face) < 0.1` or the interpolation degenerates.
- **Hash** the own triangles' normals in `tile_seed` (only when present), instead of bumping `BAKER_VERSION`:
  v1 results and cache entries stay valid.

## Risks / Trade-offs

- [Interpolated normals point below the surface near silhouettes → self-occlusion] → rays already start lifted
  along the normal; the face-normal fallback catches flipped data. Measured on the curved-quad test.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none. ADRs: none (implements ADR-0004 decision 2).
