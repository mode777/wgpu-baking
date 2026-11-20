# ADR-0005: Ray-traced gathering with irradiance-cache-style interpolation (not hemicubes)

- **Status:** accepted
- **Date:** 2026-11-20
- **Phase:** 4 / 6
- **Requirements:** REQ-GI-01, REQ-GI-06

## Context

The Witness' baker (Ignacio Castaño's articles, see [RESEARCH.md](../RESEARCH.md)) rendered
**hemicubes** with the GPU rasteriser at sample points, integrated them into irradiance, and used
**irradiance caching** to avoid sampling every texel. Upstream lightbaker instead traces
cosine-weighted rays through a BVH in WGSL compute, gathers on a coarse grid and interpolates
within charts. The brief asks for comparable visuals, not the same implementation, and to
preserve upstream's ray-tracing implementation.

## Decision

- Keep **ray-traced gathering** through the BVH as the only GI estimator.
- Adopt the **ideas** of irradiance caching on top of upstream's grid: record harmonic-mean hit
  distance per gather sample, refine adaptively where irradiance or geometry varies, interpolate
  with Ward-style weights restricted to the same chart and a validity radius
  ([LIGHTING_MODEL.md §5](../LIGHTING_MODEL.md#5-interpolation-and-irradiance-caching)).
- Hemicube rendering is not implemented.

## Alternatives considered

| Option | Pros | Cons |
|---|---|---|
| Hemicube rasterisation in WebGPU | Faithful to The Witness; uses raster hardware | Second renderer of the scene inside the baker; many render passes per sample; aliasing/near-plane issues Castaño describes; duplicates upstream's tracing |
| Path tracing every texel, no interpolation | Simplest, unbiased | Too slow for interactive browser bakes at Balanced quality |

## Consequences

Interpolation quality depends on good refinement criteria; the `leak` and `bounces` test scenes
guard against artefacts. "Interpolation off" remains available as a ground-truth reference.

## Verification

Phase 6: adaptive mode reaches the `grid` mode's error (vs. "off") at fewer gather samples, and
passes the `leak` scene.
