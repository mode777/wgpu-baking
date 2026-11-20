# Spec Delta

## Purpose

The `.lbscene` scene file: versions 1 and 2, their validation, and the guarantee that Rust and TypeScript read
and write the same files.

## ADDED Requirements

### Requirement: Version 1 stays readable
The engine and the TypeScript reader SHALL read `.lbscene` version 1 files as before, with identical bake results.

#### Scenario: upstream-basic v1
- **WHEN** `test-scenes/upstream-basic/upstream-basic.lbscene` (v1) is baked
- **THEN** the `.lbake` bytes equal those of the previous engine version

### Requirement: Version 2 meshes
A version 2 file SHALL describe each mesh by a vertex count and a list of typed arrays (positions required;
normals, indices, per-triangle material ids, vertex colours and lightmap UVs optional), and the readers SHALL
reject arrays of the wrong length, indices out of range and material ids without a material.

#### Scenario: Index out of range
- **WHEN** a v2 mesh has an index equal to its vertex count
- **THEN** reading fails with an error naming the mesh

### Requirement: Cross-language compatibility
A v2 file written by Rust SHALL be read by TypeScript with the same header and arrays, and vice versa.

#### Scenario: Golden files
- **WHEN** the tests read `test-scenes/format-v2/rust.lbscene` in TypeScript and `ts.lbscene` in Rust
- **THEN** both describe the same meshes, materials and arrays as the writer's input

### Requirement: Same light for the same triangles
A scene SHALL bake to the same lightmap regardless of whether its triangles are given as a v1 file or as an
indexed v2 file with the same triangles and materials.

#### Scenario: upstream-basic as indexed v2
- **WHEN** `upstream-basic` is converted to indexed v2 and baked
- **THEN** its direct and indirect atlases equal the v1 bake's

### Requirement: Unsupported lighting inputs are refused
Until the engine supports them, a bake SHALL fail with an error naming the feature when the scene has lights,
an emissive material or a double-sided material.

#### Scenario: Emissive material before P4.2
- **WHEN** a scene with a material whose emission is not black is baked
- **THEN** the bake fails with an error naming the material and saying emission is not supported yet
