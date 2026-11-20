# Tasks

## 1. Adapter policy

- [x] 1.1 Add `pub software: bool` to `Gpu` (device type Cpu or known software names) and include "software" in `Gpu::adapter`; verify `cargo build -p lightbaker` and `cargo build -p lightbaker --target wasm32-unknown-unknown` status is unchanged from before (wasm build is expected to fail until P2.1; record it)
- [x] 1.2 Update `fn gpu()` in `tests/bake.rs` for `LIGHTBAKER_REQUIRE_GPU` / `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER`, printing the adapter; verify locally: `cargo test -p lightbaker --release` runs on the Intel GPU (not skipped) and with `WGPU_ADAPTER_NAME=nonexistent`/unavailable backend + `LIGHTBAKER_REQUIRE_GPU=1` it fails

## 2. Workflow

- [x] 2.1 Add `.github/workflows/ci.yml` (`rust-native` ubuntu+windows; `rust-gpu` ubuntu+lavapipe); verify the YAML parses and the Windows `rust-native` steps pass when run locally
- [x] 2.2 Push and verify on GitHub that `rust-native` (both OSes) and `rust-gpu` pass, and the `rust-gpu` log shows the test ran on llvmpipe (not skipped)

## 3. Docs

- [x] 3.1 Update BUILD.md (CI section: env vars, how to reproduce locally) and TESTING.md (adapter policy); tick P1.3 in ROADMAP.md

## Workflow follow-up

- Archive (sync `ci-pipeline` spec), commit on `main`, push.
