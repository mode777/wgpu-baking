# lightmap-unwrap Specification

## Purpose
Automatic lightmap UVs per object tile (xatlas): what an unwrap returns and the validity guarantees the bake
relies on.

## Requirements

### Requirement: Unwrap per object tile
Unwrapping an object tile SHALL return, per input mesh, per-vertex lightmap UVs in tile texels, the input vertex
each output vertex came from (`xref`), a new index buffer, a chart id per vertex, and the tile's width and height
in texels, sized from the requested texels per metre and padding.

#### Scenario: Unit quad
- **WHEN** a 1 m × 1 m quad is unwrapped at 32 texels per metre with padding 2
- **THEN** the tile is roughly 32 + 2 × 2 texels on its longer side and all UVs lie inside it

### Requirement: Valid lightmap UVs
Every output triangle with non-zero 3D area SHALL have non-zero UV area with consistent orientation within its
chart, all UVs SHALL lie within the tile, `xref` SHALL map every output vertex to an input vertex at the same
position, and no chart SHALL contain faces of different materials.

#### Scenario: Multi-material mesh
- **WHEN** a mesh with three material ids is unwrapped
- **THEN** every chart's faces share one material id

#### Scenario: Validity on test meshes
- **WHEN** the test meshes (upstream-basic meshes, sphere, torus knot) are unwrapped
- **THEN** all validity checks pass
