# Tasks

## 1. Contract

- [x] 1.1 `lightbaker::unwrap`: trait, input/option/output types, `check_uvs` + unit tests; verify `cargo test -p lightbaker --lib unwrap` and wasm32 build

## 2. Native xatlas

- [x] 2.1 `crates/lightbaker-uv`: vendored xatlas + LICENSE, `build.rs` (`cc`, C API, single-threaded; no-op on wasm32), FFI, `XatlasUnwrapper`; sizeof probe test; verify `cargo build -p lightbaker-uv` and `--target wasm32-unknown-unknown`
- [x] 2.2 UV validity suite (quad sizing, 3-material box, upstream-basic meshes, sphere, torus knot, determinism); verify `cargo test -p lightbaker-uv`, clippy + fmt clean

## 3. CI and docs

- [x] 3.1 CI: `lightbaker-uv` in `CLIPPY_CRATES`, wasm32 build in the `wasm` job; THIRD_PARTY_NOTICES (xatlas vendored); BUILD.md (C++ compiler), AGENTS.md/ARCHITECTURE §4/ADR-0006 (contract in `lightbaker::unwrap`); tick P3.5 in ROADMAP.md

## Workflow follow-up

- Archive (sync `lightmap-unwrap`), commit on `main`, push, confirm CI green (Ubuntu gcc and Windows MSVC build the C++).
