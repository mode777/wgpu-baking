# Tasks

## 1. Engine hooks and GPU robustness

- [x] 1.1 Add `progress.rs` (`Progress`, `Phase`, `Hooks`, `Cancelled`), `Options::{progress, cancel}` + `Default`; update all `Options` literals; verify `cargo build --workspace --all-targets`
- [x] 1.2 `Gpu::run`/`bake`/`bake_volume`: progress per batch and stage, cancel checks; GPU test: phases in order ending at done = total, cancel after the first `direct` report fails with `Cancelled` and a following bake gives identical bytes; verify on the Intel GPU
- [x] 1.3 Platform selection by `target_arch`; uncaptured-error and device-lost capture in `Gpu` with `GpuError::{DeviceLost, Gpu}` and `set_device_lost_callback`; `Gpu::probe()`; verify native tests and wasm32 build

## 2. Bridge crate

- [x] 2.1 Create `crates/lightbaker-web` (workspace member, cdylib+rlib, deps) with `config.rs` and `errors.rs` and host unit tests; verify `cargo test -p lightbaker-web`
- [x] 2.2 Implement `capabilities`, `WebBaker` (`create`, `adapter`, `bake`, `cancel`, `evaluate`, `clearCache`, `setDeviceLostCallback`); verify `wasm-pack build crates/lightbaker-web --target web` and clippy `-D warnings`
- [x] 2.3 wasm-bindgen tests (Node): `capabilities()` → `webgpu: false` with a reason; `WebBaker.create(false)` rejects with kind `no-webgpu`; verify `wasm-pack test --node crates/lightbaker-web`

## 3. CI and docs

- [x] 3.1 CI: `lightbaker-web` in `CLIPPY_CRATES`; `wasm` job runs `wasm-pack build` and `wasm-pack test --node`; verify locally, then on GitHub
- [x] 3.2 Update INTERFACES.md §7–9 (implemented API, `cancelled` kind, result shape), BUILD.md (wasm-pack commands verified, versions), regenerate THIRD_PARTY_NOTICES.md; tick P2.4 in ROADMAP.md

## Workflow follow-up

- Archive (sync `wasm-bridge`, `bake-control`, `gpu-access`), commit on `main`, push, confirm CI green.
