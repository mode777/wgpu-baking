# Tasks

## 1. Rust

- [x] 1.1 `scene.rs`: v2 header fields with defaults, array grammar, `FileMesh`/`SceneFile`, reader for v1+v2 (split by material, `origin`, expanded attributes), `write_v2`, `to_bytes`; unit tests (round trip, index out of range, bad array length, unknown array, material split); verify `cargo test -p lightbaker --lib scene`
- [x] 1.2 `bake()` refuses lights / emission / doubleSided with named errors; bridge uses `to_bytes`; GPU test: upstream-basic as indexed v2 bakes to the same direct/indirect atlases as v1, v1 `.lbake` bytes unchanged; verify on the Intel GPU and wasm32 build

## 2. TypeScript

- [x] 2.1 `js/src/scene.ts`: v2 types, `writeSceneV2`, `readScene` v1+v2 with validation; Vitest (round trip, errors)

## 3. Golden files and docs

- [x] 3.1 `test-scenes/format-v2/{rust,ts}.lbscene` with cross-reading tests in both languages
- [x] 3.2 docs/FORMAT.md (v2), INTERFACES.md §5 (as implemented), ADR-0010; tick P3.2 in ROADMAP.md

## Workflow follow-up

- Archive (sync `scene-format`), commit on `main`, push, confirm CI green.
