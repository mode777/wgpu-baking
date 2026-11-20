# Spec Delta

## Purpose

How lightmap texels are found on triangles (UV-space rasterisation with conservative coverage) and which surface
point and normal each texel represents.

## ADDED Requirements

### Requirement: Rasterisation from triangle UVs
A tile's texels SHALL be found by rasterising each triangle at its tile-local lightmap UVs: first every texel whose
centre it covers, then every texel it only touches; each texel's point, and its step to the next texel along x and
y, SHALL come from the triangle's UV→surface mapping.

#### Scenario: Planar layout unchanged
- **WHEN** `upstream-basic` (v1) is baked
- **THEN** the `.lbake` bytes equal those of the previous version

### Requirement: Interpolated vertex normals
When a mesh has vertex normals, a texel's normal SHALL be the normalised barycentric interpolation of its
triangle's vertex normals; without them, or if the interpolated normal faces away from the triangle, the face
normal SHALL be used.

#### Scenario: Curved quad
- **WHEN** a quad whose vertex normals tilt by ±30° across it is rasterised
- **THEN** texel normals vary smoothly between the corner normals and stay unit length

#### Scenario: No normals
- **WHEN** the mesh has no normals
- **THEN** every texel of a triangle carries its face normal

### Requirement: Normals in tile hashes
A tile's cache hash SHALL change when the normals of its own triangles change.

#### Scenario: Normal edit
- **WHEN** only a vertex normal of a tile's triangle changes between two bakes with the same store
- **THEN** that tile is baked again
