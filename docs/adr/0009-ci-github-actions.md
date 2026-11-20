# ADR-0009: CI on GitHub Actions with software GPU adapters

- **Status:** accepted
- **Date:** 2026-11-20
- **Phase:** 1–2 (extended in 3–5)
- **Requirements:** REQ-ENG-03, REQ-ENG-04, REQ-ENG-05, REQ-WASM-04

## Context

Standard GitHub-hosted runners have no GPU. GPU "larger runners" (Tesla T4, Ubuntu/Windows) exist
but are billed per minute (Team/Enterprise plans; public repositories are billed too). Evidence that
software GPUs suffice for correctness tests:

- **wgpu's own CI** runs its GPU test suite on standard runners: Linux with Mesa **lavapipe**
  (software Vulkan), Windows with **WARP** (software D3D12) and Mesa, macOS with Metal
  (`gfx-rs/wgpu/.github/workflows/ci.yml`, `install-mesa`, `install-warp` actions).
- **three.js** runs its WebGPU examples — compute examples included — in Chrome on `ubuntu-latest`
  with only `apt-get install mesa-vulkan-drivers xvfb` and Chrome flags
  `--enable-unsafe-webgpu --enable-features=Vulkan --disable-vulkan-surface --ignore-gpu-blocklist
  --disable-gpu-watchdog --no-sandbox`, headed under `xvfb-run`
  (`mrdoob/three.js/.github/workflows/ci.yml`, `test/e2e/puppeteer.js`).

The project forbids silent baking on software adapters (AGENTS.md rule 2), so CI must opt in
explicitly.

## Decision

Free standard runners only; correctness on software adapters; performance measured manually.

| Workflow job | Runner | Contents | Budget |
|---|---|---|---|
| `rust-native` | `ubuntu-latest`, `windows-latest` | fmt/clippy (new crates), build, unit tests | < 10 min |
| `rust-gpu` | `ubuntu-latest` + `mesa-vulkan-drivers` (lavapipe) | `cargo test -p lightbaker` GPU integration tests with `LIGHTBAKER_REQUIRE_GPU=1` (no-adapter = **fail**, not skip) and `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1`; CI-sized scenes | < 10 min |
| `wasm` | `ubuntu-latest` | `cargo build --target wasm32-unknown-unknown --features web`, `wasm-pack build`, `wasm-bindgen-test` (Node, non-GPU units) | < 10 min |
| `web` | `ubuntu-latest` + `mesa-vulkan-drivers` + `xvfb` | pnpm install, typecheck, Vitest, app build, then the **small** Playwright set (see TESTING.md) in Chrome with the flags above and `allowFallbackAdapter: true` | < 10 min |
| `gpu-perf` (optional) | GPU larger runner (T4) or self-hosted, `workflow_dispatch` only | timing benchmarks for phase reports | manual |

Rules:

- Software-adapter results are labelled (adapter name in logs and test output). Software adapters
  are recognised by `isFallbackAdapter` **or** adapter names (llvmpipe/lavapipe, SwiftShader,
  "Microsoft Basic Render Driver"/WARP).
- CI scenes and the `ci` bake profile are small (low ray counts, ≤ 512² atlas) so lavapipe runs
  them in seconds; assertions are metric-based with tolerances.
- No image-diff visual regression and no browser/GPU matrix in CI; those are manual phase-report
  activities.
- Network-dependent tests (polyfork CDN) never run in CI.

## Alternatives considered

| Option | Why not (as default) |
|---|---|
| GPU larger runners for every push | Cost; software adapters already prove correctness. Kept as optional `gpu-perf`. |
| Self-hosted GPU runner | Maintenance and security burden for a public repo; optional later. |
| SwiftShader instead of lavapipe in Chrome | Possible; lavapipe is what three.js uses successfully — start there. |

## Consequences

- **Finding (P2.6):** on `ubuntu-latest` without a GPU, Chromium and Google Chrome expose **only their
  built-in SwiftShader WebGPU adapter** (`google/swiftshader`, `isFallbackAdapter`), with or without
  `--enable-features=Vulkan`, `--use-vulkan=native`, `VK_DRIVER_FILES` pointing at lavapipe, etc.
  (`app/scripts/adapter-probe.mjs`, CI run 44765129072). The browser job therefore bakes on SwiftShader
  (a Vulkan implementation on the CPU), the native `rust-gpu` job on lavapipe. Both are software
  adapters, explicitly allowed and labelled; the Phase 2 exit criterion "on lavapipe in CI" is met in
  that sense (native: lavapipe; browser: Chrome's software adapter).

- Timing numbers from CI are meaningless; performance targets are checked on real GPUs manually
  (phase reports) or via `gpu-perf`.
- Lavapipe/WARP may hide driver-specific bugs on real GPUs → phase reports include real-GPU runs.

## Verification

P1.3: `rust-gpu` job runs the upstream GPU test on lavapipe (not skipped). P2.6: Chrome on
lavapipe bakes the CI-sized `upstream-basic` scene in the `web` job.
