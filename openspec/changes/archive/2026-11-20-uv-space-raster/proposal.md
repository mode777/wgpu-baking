# Proposal: uv-space-raster (P3.4)

## Why

Texels are found by rasterising each triangle in its chart's tile through the planar chart's own projection
(`Chart::local`), and every texel gets the flat face normal. xatlas charts are not planar (ADR-0004 decision 2):
rasterisation must take each triangle's tile-local UVs from wherever the unwrap put them, and curved assets need
interpolated vertex normals (now available from `.lbscene` v2, P3.2) or they bake faceted. Requirements:
REQ-UV-04, REQ-UV-06 (no seams from faceting), REQ-GI-01.

## What Changes

- `raster.rs` takes each triangle's tile-local UVs from one layout function (`Layout::triangle_uvs`); for the
  planar layout it evaluates the chart projection as before, so v1 results are unchanged byte for byte.
- Texel normals are the interpolated, normalised vertex normals when the mesh has normals (v2), else the face
  normal; a vertex normal facing away from its face (bad data) falls back to the face normal.
- Tile hashes include the normals of a tile's own triangles, so cached tiles are not reused across a normal
  change.

## Capabilities

### New Capabilities

- `lightmap-raster`: how lightmap texels are found on triangles (UV-space rasterisation, conservative coverage)
  and which surface point and normal each texel represents.

### Modified Capabilities

None.

## Out of scope

xatlas UVs feeding the layout (P3.6/P3.7), normal maps, smoothing groups beyond what the file's normals say.

## Impact

`raster.rs`, `layout.rs` (`triangle_uvs`), `bake.rs` (`tile_seed`), tests (`tests/raster.rs`).
