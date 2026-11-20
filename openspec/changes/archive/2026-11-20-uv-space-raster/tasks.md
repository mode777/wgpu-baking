# Tasks

## 1. Raster

- [x] 1.1 `Layout::triangle_uvs`; raster uses it; interpolated vertex normals with face fallback; normals in `tile_seed`; verify v1 `.lbake` bytes unchanged and all GPU tests pass
- [x] 1.2 `tests/raster.rs` (CPU): curved quad (smooth, unit normals, between corners), no-normals quad (face normal), flipped vertex normal falls back; GPU test: normal edit rebakes the tile with a `MemoryStore`; verify

## 2. Docs

- [x] 2.1 ARCHITECTURE §3.1 raster note, ADR-0004 status (decision 2); tick P3.4 in ROADMAP.md

## Workflow follow-up

- Archive (sync `lightmap-raster`), commit on `main`, push, confirm CI green.
