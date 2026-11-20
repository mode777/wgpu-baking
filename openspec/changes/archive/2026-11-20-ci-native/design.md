# Design: ci-native

## Context

Upstream's `fn gpu()` in `tests/bake.rs` returns `None` on any `Gpu::new()` error. `Gpu` exposes only
`adapter: String` = `"{name} ({backend:?})"`; `Gpu::open` picks `HighPerformance` and requests the
adapter's full limits. No CI exists.

## Goals / Non-Goals

**Goals:** green CI that provably runs the GPU test on lavapipe; Windows coverage of the build.
**Non-Goals:** engine-side adapter policy (P2.2), wasm/web jobs.

## Decisions

- **Policy in the test helper, detection by name + device type.** `Gpu` already has the adapter name;
  to also see the device type without changing the engine API we add one additive, read-only field
  `Gpu::software: bool` set in `open()` from `AdapterInfo.device_type == Cpu` or the known software names
  (same list as ADR-0009). Alternative: name matching only in the test — rejected because lavapipe
  reports itself as `llvmpipe (LLVM …)` and WARP as "Microsoft Basic Render Driver"; device type catches
  unknown software adapters too. The field is reused by P2.2's adapter report.
- **One workflow file, two jobs**, triggered on `push` to `main` and `pull_request`. `Swatinem/rust-cache`
  keeps jobs under ADR-0009's 10-minute budget. The toolchain comes from `rust-toolchain.toml`
  (`dtolnay/rust-toolchain` is not needed: rustup on runners honours the file).
- **lavapipe install:** `sudo apt-get install -y mesa-vulkan-drivers` (as wgpu's and three.js' CI);
  `VK_ICD_FILENAMES` is not set so the loader finds lavapipe's ICD; `vulkaninfo --summary` is logged for
  diagnosis.
- **Release-mode tests in `rust-gpu`** (`cargo test --release`): lavapipe runs the test's 256-ray gather;
  the debug-mode CPU stages (BVH, filters) dominate otherwise.
- **Clippy only for new crates:** an env var `CLIPPY_CRATES` (empty now) lists crates linted with
  `-D warnings`; upstream crates are not linted (ADR-0001: no wholesale reformatting).
- **Local CI:** no Docker/WSL on the development machine; the same commands are run natively on Windows
  (the `rust-native` Windows job verbatim, `rust-gpu` against the Intel GPU), Linux jobs are verified on
  GitHub after push.

## Risks / Trade-offs

- [lavapipe too slow for the upstream scene] → release mode; if still > 10 min, a CI-sized variant comes
  with P1.4.
- [Upstream test also bakes the volume and a moved-box rebake] → acceptable runtime; measured in the first run.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none new (one bool on `Gpu`).
ADRs: none created (implements ADR-0009).
