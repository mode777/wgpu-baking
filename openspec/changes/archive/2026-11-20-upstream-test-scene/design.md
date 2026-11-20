# Design: upstream-test-scene

## Context

`tests/bake.rs` builds the scene (`quad`, `cube`, `scene(bx)`), locates texels with `ground_texel` via
`Layout::tri_chart` and `Chart::affine`, and asserts on `Bake::{direct, indirect, volume}`.

## Goals / Non-Goals

**Goals:** one source for the scene; expectations as data, evaluable from Rust now and reusable by the
web bridge later (the module is platform-agnostic). **Non-Goals:** a general expression language.

## Decisions

- **Shared builder via `#[path]`.** `crates/lightbaker/tests/common/upstream_basic.rs` holds `quad`, `cube`,
  `scene`; `tests/bake.rs` and `examples/export-upstream-basic.rs` include it with `#[path = …] mod`. The
  upstream test keeps its assertions; only the builder moves (smallest diff that avoids duplication).
  Alternative: copy the builder into the example — rejected, the committed scene could drift from the test.
- **Probe sampling in the engine crate (`lightbaker::expect`)**, because it needs `Layout` internals (charts,
  affine) and must later run inside wasm next to the bake. Point → triangle: the triangle of the given mesh
  whose plane is within 1 mm of the point and whose barycentrics are within [-1e-4, 1+1e-4]; then the chart
  affine gives the atlas texel; the indirect texel is the atlas texel divided by the downscale. Volume
  probes use the nearest-lower grid probe as the upstream test does.
- **Sample vector per map** (4 components so checks are uniform): direct `[field, ao, 0, 0]`; indirect
  `[r, g, b, r+g+b]`; volume `[L0 r, L0 g, L0 b, sun]`.
- **Operands**: a number, `name[i]`, or `name[i]/name[j]`. Check: `a op b·scale + offset`. This expresses
  every upstream assertion (e.g. bleed ratio `near[0]/near[1] > far[0]/far[1]·1.15`).
- **Reference values** (`reference.values`, adapter name) are written by the exporter when it can bake
  (`--bake`), for native-vs-browser comparison in Phase 2. They are informative, not checks.
- **Reproducibility test**: bake twice without cache, compare `output::write` bytes (upstream already
  compares cached rerun vs fresh).

## Risks / Trade-offs

- [Probe on a chart/tile boundary picks a different texel across adapters] → probes copied from upstream,
  which are inside charts.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none (pure CPU sampling of existing results;
the exporter/test use the existing `Gpu::new`, native only). ADRs: none; the `expect.json` format is a test
artefact documented in TESTING.md.
