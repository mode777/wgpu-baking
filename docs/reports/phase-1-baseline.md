# Phase 1 report — native baseline

- **Roadmap:** Phase 1 "Understand and fork" (P1.1–P1.5), milestone M1.
- **Date:** 2026-11-20
- **Upstream:** jer-nc/lightbaker `4fe85da`, merged with full history (`git merge upstream/main` is a no-op).
- **Fork:** <https://github.com/mode777/wgpu-baking>, `main`.

## Exit criteria

| # | Criterion | Result | Evidence |
|---|---|---|---|
| 1 | `cargo test -p lightbaker` passes natively with a GPU, not skipped — CI on lavapipe and locally on a hardware GPU | **met** | Local: Intel(R) Graphics, Vulkan and DX12, `LIGHTBAKER_REQUIRE_GPU=1`, all tests pass (below). CI `rust-gpu`: `llvmpipe (LLVM 21.1.8, 256 bits) (Vulkan, software)`, `LIGHTBAKER_REQUIRE_GPU=1`, all tests pass, adapter labelled as software. |
| 2 | `lightbaker-cli` bakes `test-scenes/upstream-basic` reproducibly (identical bytes twice) | **met** | Two CLI runs → identical `.lbake` (BLAKE3/SHA-256 equal); `tests/scenes.rs` asserts it on every GPU run; `--check` passes all 19 checks. |
| 3 | Baseline report committed; FEASIBILITY.md corrected where it differed | **met** | This file; FEASIBILITY.md §3 (B5 counts) and §5 (measured transfers, `adapter.limits()`) corrected. |

## Machines and adapters

| Where | Adapter | Backend | Notes |
|---|---|---|---|
| Local (Windows 11 Enterprise, Intel Core Ultra 5 335, 31.5 GiB RAM) | Intel(R) Graphics, driver 32.0.101.8831 | Vulkan (default pick), DX12 (`WGPU_BACKEND=dx12`) | hardware, integrated |
| GitHub Actions `ubuntu-latest` | llvmpipe (LLVM 21.1.8, 256 bits) | Vulkan (Mesa lavapipe) | **software**, opt-in via `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1` |

## Tests

`$env:LIGHTBAKER_REQUIRE_GPU='1'; cargo test -p lightbaker --release -- --nocapture` (Intel, Vulkan):

| Test | Result |
|---|---|
| unit (`expect` parsing, operands, point-on-triangle; upstream unit tests) | 5 passed |
| `adapter_policy` (software adapter recognition) | 1 passed |
| `bake` (upstream: shadow, AO, red bleed, volume, strip, cache reuse, moved-box rebake) | 1 passed, 0.79 s |
| `scenes` (committed scene = generator; two bakes identical; 19 checks; probe-off-mesh error; transfer counters) | 2 passed, 0.52 s |

The same suite passes on lavapipe in CI (`rust-gpu` job).

## Baseline measurements

Command: `cargo run -p lightbaker --release --example baseline -- --json <file>` (median of 5 bakes after a
warm-up, no tile cache). Stage times are the gaps between the bake's log lines (each stage logs when it
ends). Raw data: [phase-1-baseline-intel.json](phase-1-baseline-intel.json) (Vulkan),
[phase-1-baseline-intel-dx12.json](phase-1-baseline-intel-dx12.json) (DX12).

Scenes: `upstream-basic` (538 triangles, 4 texels/m, atlas 1024×180, 256 GI rays, 2 bounces, 33×3×33
volume) and `upstream-basic-dense` (same geometry, 16 texels/m, atlas 2048×924, 512 rays, 3 bounces).

### Intel(R) Graphics — median bake time per stage (ms)

| Stage | basic, Vulkan | basic, DX12 | dense, Vulkan | dense, DX12 |
|---|---:|---:|---:|---:|
| layout + BVH + GPU scene upload | 0.4 | 0.8 | 0.6 | 0.5 |
| tile hashes + cache lookup | 1.8 | 1.9 | 2.1 | 2.0 |
| rasterise + direct pass (GPU) | 15.9 | 15.3 | 197.4 | 157.5 |
| uncovered-chart check | 0.7 | 0.7 | 8.6 | 9.8 |
| CPU per-tile direct (sun field, AO, filters) + bounce 1 gather (GPU) | 23.0 | 23.4 | 404.9 | 399.9 |
| bounce 2 gather (GPU) | 11.4 | 8.4 | 162.9 | 181.6 |
| bounce 3 gather (GPU) | — | — | 159.8 | 180.0 |
| volume (GPU) | 6.8 | 5.7 | 8.9 | 10.3 |
| atlas assembly | 4.6 | 5.6 | 80.9 | 80.3 |
| **total** | **64.6** | **60.1** | **1060.7** | **1049.7** |

Covered texels: basic 71 439 (direct), 18 518 gathered per bounce; dense 1 112 074 / 280 579.

### lavapipe (CI, informational)

Run 44751798793 (`rust-gpu` job, artifact `baseline-lavapipe`), llvmpipe (LLVM 21.1.8, 256 bits):

| Stage | basic | dense |
|---|---:|---:|
| layout + BVH + GPU scene upload | 0.4 | 0.5 |
| tile hashes + cache lookup | 1.4 | 1.4 |
| rasterise + direct pass (GPU) | 251.1 | 3133.6 |
| uncovered-chart check | 0.4 | 5.9 |
| CPU per-tile direct + bounce 1 gather (GPU) | 174.3 | 3769.0 |
| bounce 2 gather (GPU) | 139.4 | 3330.5 |
| bounce 3 gather (GPU) | — | 3346.2 |
| volume (GPU) | 41.0 | 36.8 |
| atlas assembly | 3.1 | 29.8 |
| **total** | **598.7** | **13665.7** |

Peak resident memory (VmHWM, whole process incl. the software rasteriser): 555.6 MiB. lavapipe is
≈ 10× (basic) to ≈ 13× (dense) slower than the Intel iGPU: CI must keep to `upstream-basic`-sized scenes
(ADR-0009's `ci` profile).

### CPU ↔ GPU transfers per bake

| Scene | Dispatch batches | Readbacks | Read back | Uploaded |
|---|---:|---:|---:|---:|
| upstream-basic | 7 | 7 | 1.85 MiB | 15.29 MiB |
| upstream-basic-dense | 72 | 72 | 30.01 MiB | 234.99 MiB |

Identical on every adapter (they depend only on the scene). Uploads are dominated by the `light` atlas
(16 B/texel: 2.8 MiB basic, 28.9 MiB dense) and the `texels` array (64 B/texel), both re-uploaded on
each `Gpu::run` call (direct, each bounce, volume).

### Memory

- Peak working set of the `baseline` process on Windows (both scenes, Vulkan): **≈ 800 MiB**, measured
  with `Get-Process` PeakWorkingSet64. This includes the Vulkan driver's mappings; the bake's own large
  arrays for the dense scene are ≈ 29 MiB (`light`), ≈ 68 MiB (`texels`), plus per-tile arrays.
- Peak resident memory on lavapipe (VmHWM, includes the software rasteriser): 555.6 MiB.
- Estimated GPU buffers for the dense scene: `light` 28.9 MiB, `texels` ≈ 68 MiB, BVH and triangles
  < 50 KiB, per-batch `out`/`read` ≤ 1 MiB each.

## Observations for Phase 2

1. **The CPU stages matter.** In the dense scene, "CPU per-tile direct + bounce 1" (≈ 400 ms) is far
   above the GPU-only bounces (≈ 160–180 ms), so most of it is the CPU sun field/AO/filter work, which
   runs with rayon natively and will run **sequentially** on the web (B5). Expect the browser to be
   slower here even with equal GPU speed.
2. **Uploads dominate transfers**: 235 MiB per dense bake, mostly the re-uploaded `light` atlas and
   texel arrays. Fine for Phase 2; Phase 6 (REQ-WASM-05) keeps them on the GPU.
3. **Readbacks are few and batch-sized** (≤ 1 MiB each): the async readback in P2.2 adds one await per
   batch (7 / 72 per bake), not per texel.
4. Vulkan and DX12 on the same Intel GPU are within a few percent; Chrome on Windows uses D3D12 (Dawn),
   so the DX12 column is the closer native comparison for Phase 2.
5. `device` is requested with the adapter's full limits; for the web, P2.2 must request explicit
   limits (the dense `light` atlas, 28.9 MiB, is within WebGPU's default 128 MiB binding size).

## FEASIBILITY.md verification

Checked against the running code at the fork point:

| Claim | Verified |
|---|---|
| §2 inventory: file sizes, roles; `BAKER_VERSION = 11`; `wgpu 30.0.1`, `glam 0.34.3`, `bytemuck 1.25.2`, `blake3 1.8.7`, `flate2 1.1.11`/`miniz_oxide 0.9`, `rayon 1.12`, `pollster 1.0.1`, `zstd 0.14.3`/`zstd-sys 2.1.2` | ✓ |
| §3 B1–B4, B6–B9 locations | ✓ (`pollster::block_on` in `Gpu::new`, `mpsc` + `PollType::wait_indefinitely` in `run`, `new_without_display_handle_from_env`, `Instant` ×3 in `bake.rs` + `volume.rs`, `std::fs` + `zstd` in `cache.rs`, `zstd` in `encode.rs`) |
| §3 B5 count | **corrected**: `bake.rs` has `par_iter` ×3 + `par_iter_mut` ×1, `deps.rs` `par_iter` ×1 (5 in total, not 5 in `bake.rs`) |
| §3 wasm32 build | ✓ fails first at `zstd-sys` (B7), as expected |
| §5 bindings: 7 storage + 1 uniform; `Tri` 48 B, `Node` 32 B, `Texel` 64 B; batches 65 536 / 16 384 / 8 192; texel slices bound at `first × 64` | ✓ |
| §5 readbacks/uploads | **extended** with measured numbers and the per-call `texels` re-upload; device requests `adapter.limits()` |

## Changes made in Phase 1

- P1.1 fork with history, upstream README → `docs/UPSTREAM_README.md`, public repo.
- P1.2 `rust-toolchain.toml` (1.99.1 + wasm32), ignores, `THIRD_PARTY_NOTICES.md` (generated), LICENSE line.
- P1.3 CI (`rust-native` Ubuntu/Windows, `rust-gpu` lavapipe); GPU test adapter policy; `Gpu::software`.
- P1.4 `test-scenes/upstream-basic` + `expect.json`, `lightbaker::expect`, CLI `--check`.
- P1.5 `Gpu` transfer counters, `baseline` example, this report.

Upstream files touched (all additive): `gpu/mod.rs` (`software`, transfer counters), `lib.rs`
(`mod expect`), `tests/bake.rs` (shared builder/helper), `lightbaker-cli/src/main.rs` (`--check`),
`Cargo.toml` (repository URL), `.gitignore`, `.gitattributes`, `LICENSE` (one line).
