## Context

`xatlas-wasm` is an Emscripten module loaded asynchronously; after loading, all calls are synchronous.
The bridge (`lightbaker-web`, wasm-bindgen) cannot link it, but can call a JS object handed to it. The
worker already has a TS adapter with the contract's shape (`unwrapTile(meshes, options)`).

## Goals / Non-Goals

**Goals:** the Rust unwrap contract reachable in the browser; one implementation of "unwrap a scene"
shared by native and web; a test that native and web xatlas agree.

**Non-Goals:** using the unwrap in the bake (P3.7), `uv1` output (P3.8), caching charts (P3.7).

## Decisions

- **Adapter passed in, not imported by path.** `#[wasm_bindgen] extern "C" { type JsUnwrapper; #[wasm_bindgen(method, catch, js_name = unwrapTile)] … }`
  — any object with `unwrapTile(meshes, options)` (the TS adapter bound to the loaded module, a fake in
  tests). A `module = "…"` import would tie the wasm package to the app's file layout and to `xatlas-wasm`
  being resolvable from `pkg/`. The adapter's input/output are the contract's types as plain objects with
  typed arrays (camelCase).
- **`unwrap_scene` in the engine**, generic over `Unwrapper`: each engine mesh (v2 meshes are split per
  material) is one tile; input is the mesh's triangle soup plus vertex normals if the scene has them; output
  per mesh: the tile and `check_uvs`' report. The bridge converts it to the existing `UnwrapResult` shape
  (plus the report), so the worker protocol and page are unchanged.
- **Equivalence by fixture.** An example (`lightbaker-uv`, `export-uv-equivalence`) writes
  `test-scenes/uv-equivalence/{uv-equivalence.lbscene, native.json}`: a quad, a three-material box, a UV
  sphere and a small torus knot, and their native unwraps (tile size, charts, `xref`, indices, UVs). The
  Rust test and the app test (bridge + `xatlas-wasm` in Node via `initSync`) unwrap the scene and compare.
  Tolerance is decided by measurement: xatlas is deterministic per build, but MSVC/gcc/clang and different
  libm `sinf`/`acosf` can move UVs by rounding; topology (tile size, charts, `xref`, indices) must match and
  UVs must agree within 0.01 texel, else the difference is documented and the comparison relaxed to
  validity + tile size + chart count.

## Risks / Trade-offs

- [Cross-compiler float differences change chart decisions] → measured on Windows MSVC, Linux gcc and wasm;
  the fixture tolerance is set from that and documented in ADR-0006.
- [Copy cost across the JS boundary] → one copy each way per tile (P3.1 measured < 1 ms per 10 k vertices).
