# Proposal: per-triangle-uvs (P3.3)

## Why

Bounced light is looked up through each planar chart's affine world→atlas map (`charts` buffer, `Tri.e1.w`).
xatlas charts are not planar and have no such map (ADR-0004), so the GPU must find a hit's lightmap texel from
per-triangle UVs instead. This change makes the existing planar layout use that path, with unchanged metrics, so
xatlas can plug in later. Requirements: REQ-UV-04 (shared downstream path), REQ-GI-01 (unchanged GI),
REQ-WASM-08 (bindings within WebGPU limits).

## What Changes

- **GPU layout:** `Tri` grows from 48 to 80 bytes: `uv01` (u0 v0 u1 v1) and `uv2` (u2 v2, page as bits,
  reserved), lightmap UVs in atlas texels; `e1.w` becomes a reserved flags word. The `charts` storage binding is
  removed (7 bindings instead of 8).
- `trace()` returns the hit's barycentrics; `bounced()` interpolates the hit's UV.
- The planar layout emits per-triangle UVs by evaluating its affine maps at the corners (one page, page 0).
- Rust↔WGSL layout test: `Node`, `Tri`, `Texel`, `Params` sizes and member offsets compared with `trace.wgsl`
  reflected by naga.
- `BAKER_VERSION` 11 → 12 (results change by float rounding; cached tiles must not be reused).
- ADR-0004 updated: per-triangle UVs live in `Tri` (80 B) rather than a separate `tri_uv` buffer.

## Capabilities

### New Capabilities

- `gpu-layout`: the data the GPU passes read — struct layouts shared by Rust and WGSL, binding count — and the
  guarantee that both sides agree.

### Modified Capabilities

None.

## Out of scope

UV-space rasterisation (P3.4), xatlas UVs in the bake (P3.6), several pages (P3.7), output of per-vertex UVs (P3.8).

## Impact

`bvh.rs`, `gpu/mod.rs`, `gpu/trace.wgsl`, `bake.rs`, `layout.rs` (helper), `deps.rs` (`BAKER_VERSION`),
`tests/` (layout test, reference metrics), ADR-0004, ARCHITECTURE §3.3, FEASIBILITY §5 status.
