# Spec Delta

## Purpose

The data the GPU passes read — struct layouts shared by Rust and WGSL and the number of bindings — and the
guarantee that both sides agree byte for byte.

## ADDED Requirements

### Requirement: Rust and WGSL structs agree
Every struct shared between Rust and `trace.wgsl` SHALL have the same size and member offsets on both sides, and a
test SHALL compare them.

#### Scenario: Tri layout
- **WHEN** the layout test reflects `trace.wgsl`
- **THEN** `Tri` is 80 bytes in both languages with members at offsets 0, 16, 32, 48, 64

### Requirement: Bindings within the WebGPU defaults
The trace passes SHALL use at most 8 storage buffers per shader stage (WebGPU's default limit).

#### Scenario: After removing the charts buffer
- **WHEN** the bind group layout is created
- **THEN** it has 6 storage buffers and 1 uniform buffer

### Requirement: Bounced light through per-triangle UVs
The light a ray's hit bounces SHALL be read from the lightmap texel at the hit's interpolated per-triangle UV, and
the planar layout's results SHALL keep meeting every check of `upstream-basic/expect.json`, with probe values
within 2 % (or 0.02 absolute) of the recorded reference.

#### Scenario: upstream-basic metrics
- **WHEN** `upstream-basic` is baked after the change
- **THEN** all 19 checks pass and every probe value is within tolerance of `expect.json`'s reference
