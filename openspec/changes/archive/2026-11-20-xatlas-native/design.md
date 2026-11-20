# Design: xatlas-native

## Context

ADR-0006: native FFI over vendored xatlas `f700c77`, hand-written declarations, `cc`. AGENTS.md: `lightbaker`
depends on none of our crates; ARCHITECTURE put the contract in `lightbaker-uv`. P3.7 needs the engine to call an
unwrapper per object tile.

## Goals / Non-Goals

**Goals:** native xatlas behind the contract; validity checks in Rust (same rules as `app/src/bake/uvValidity.ts`).
**Non-Goals:** progress callbacks, custom allocators, xatlas' image output.

## Decisions

- **Contract in `lightbaker::unwrap`**, implementations elsewhere (`lightbaker-uv` native, `lightbaker-web` via JS,
  planar inside the engine later). Keeps the dependency rule; ADR-0006 decision 1 amended (contract location).
- **Build:** `cc` with `XATLAS_C_API=1`, `XA_MULTITHREADED=0` (same as `xatlas-wasm`, deterministic, no threads),
  C++17, `-O2`; warnings from vendored code silenced. `build.rs` does nothing on `wasm32`.
- **FFI:** `#[repr(C)]` mirrors of `xatlasMeshDecl`, `xatlasChartOptions`, `xatlasPackOptions`, `xatlasAtlas`,
  `xatlasMesh`, `xatlasVertex`; options initialised through `xatlasChartOptionsInit`/`xatlasPackOptionsInit`
  (defaults from xatlas itself), then overridden. One atlas per tile, always `xatlasDestroy` (RAII guard).
  A size test compares struct sizes with a tiny C++ `sizeof` probe compiled by the same `cc` build.
- **Output**: copies into owned `Vec`s; UVs in tile texels as xatlas gives them.
- **Validity checks** ported 1:1 from TS (out-of-range, degenerate, flipped vs chart majority, xref positions,
  mixed-material charts, density excluding charts under 4×4 texels).
- **Test meshes** generated in Rust (UV sphere, torus knot with three.js' parametrisation) — no files.

## Risks / Trade-offs

- [C++ build time (~330 KB source) in CI] → compiled once per cache; acceptable.
- [Struct layout mismatch] → the sizeof probe test.

GPU-to-CPU readbacks, blocking calls: none. Native-only dependency: the vendored C++ (via `cc`), gated off wasm32
(ADR-0006 describes the web path). ADRs: ADR-0006 updated.
