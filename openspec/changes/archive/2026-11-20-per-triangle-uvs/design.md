# Design: per-triangle-uvs

## Context

`Tri { v0 (xyz, material), e1 (xyz, chart), e2 }` (48 B); WGSL `bounced()` evaluates `charts[c*2..]` (two
affine rows) at `o + d·t`. `Bvh::build(scene, chart)` takes a chart per triangle. Bindings: 7 storage + 1 uniform.

## Goals / Non-Goals

**Goals:** one downstream path for any unwrapper; same metrics; layout guarded by a test. **Non-Goals:** pages
(P3.7, the `page` word is written as 0), packed albedo (P4.3 uses `uv2.w`).

## Decisions

- **UVs inside `Tri`** (FEASIBILITY §5) rather than ADR-0004's separate `tri_uv` buffer: one fetch per hit, no
  extra binding, and the BVH already reorders `Tri`s. 80 B keeps 16-byte alignment. ADR-0004 updated.
- **Barycentrics from Möller–Trumbore:** `p = v0 + u·e1 + v·e2`, weights `(1−u−v, u, v)` on corners
  `(v0, v1, v2)`. For a planar chart the affine map is linear, so interpolated UVs equal the affine evaluation up
  to rounding (and no longer depend on `o + d·t`'s rounding).
- **Not laid out:** `uv2.z = NONE` (was `e1.w = NONE`) → the sky fallback, as before.
- **Layout test with naga** (dev-dependency at the lock's version, `wgsl-in`): parse `trace.wgsl`, run the
  `Layouter`, compare spans and member offsets with `size_of`/`offset_of!` for `Node`, `Tri`, `Texel`, `Params`.
- **Reference metrics test:** probe values within 2 % or 0.02 of `expect.json`'s reference (Intel/Vulkan),
  so float-level changes pass and real regressions fail; it runs on lavapipe in CI too.

## Risks / Trade-offs

- [+32 B per casting triangle] → 1.6 M triangles per 128 MiB binding (FEASIBILITY §5), ample.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none (naga only as a dev-dependency).
ADRs: ADR-0004 updated.
