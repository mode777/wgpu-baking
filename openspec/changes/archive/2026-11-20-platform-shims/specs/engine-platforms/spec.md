# Spec Delta

## Purpose

Defines the platforms the baking engine builds for, what the `native` and `web` feature sets provide, and
how baked tiles are stored between bakes on each platform.

## ADDED Requirements

### Requirement: Engine compiles for the browser target
The `lightbaker` crate SHALL compile for `wasm32-unknown-unknown` with `--no-default-features --features web`,
without C dependencies, threads or file-system access.

#### Scenario: wasm32 build
- **WHEN** `cargo build -p lightbaker --target wasm32-unknown-unknown --no-default-features --features web` runs
- **THEN** it succeeds

### Requirement: Native behaviour is unchanged by the platform split
With default features, the engine SHALL produce byte-identical `.lbake` output and the same cache files as
before the split, and all existing tests SHALL pass.

#### Scenario: Native bake unchanged
- **WHEN** `test-scenes/upstream-basic` is baked natively after this change on the same adapter
- **THEN** the `.lbake` bytes equal those produced before the change

### Requirement: Pluggable tile store
A bake SHALL accept an optional tile store; tiles and probe chunks whose hash is found in it SHALL be reused,
and an in-memory store SHALL be available on every platform.

#### Scenario: In-memory reuse
- **WHEN** the same scene is baked twice with the same in-memory store
- **THEN** the second bake bakes no tiles and gives identical results

#### Scenario: No store
- **WHEN** a bake runs without a store
- **THEN** every tile is baked

### Requirement: Web output needs no zstd
Under the `web` feature, KTX2 textures in the bake output SHALL be written without supercompression
(scheme 0); under `native` they SHALL keep zstd supercompression (scheme 2).

#### Scenario: Web KTX2
- **WHEN** a KTX2 texture is encoded with the `web` feature
- **THEN** its header's supercompression scheme is 0 and its level data is the raw texel data
