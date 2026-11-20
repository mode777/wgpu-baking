# Tasks

## 1. Layout

- [x] 1.1 `Tri` 80 B (Rust) and WGSL `Tri`/`Hit` with barycentrics; `bounced()` via UVs; `charts` binding removed (Rust layout, bind group, `SceneBuffers`); `Bvh::build` takes per-triangle UVs; planar UVs from the affine maps; `BAKER_VERSION` 12; verify native build and wasm32 build
- [x] 1.2 Layout test (naga reflection of `trace.wgsl` vs `size_of`/`offset_of!`); verify `cargo test -p lightbaker --test layout`

## 2. Metrics

- [x] 2.1 Test: `upstream-basic` probe values within 2 % / 0.02 of the reference; all tests pass on the Intel GPU (and the browser e2e locally)

## 3. Docs

- [x] 3.1 ADR-0004 (UVs in `Tri`), ARCHITECTURE §3.3, FEASIBILITY §5 status; tick P3.3 in ROADMAP.md

## Workflow follow-up

- Archive (sync `gpu-layout`), commit on `main`, push, confirm CI green (lavapipe runs the reference test).
