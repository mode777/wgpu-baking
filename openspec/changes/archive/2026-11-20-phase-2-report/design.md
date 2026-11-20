# Design: phase-2-report

## Context

Native numbers exist from P1.5 (`baseline` example, Intel Vulkan/DX12, lavapipe). The browser page reports
per-phase times from progress events, total time, transfers. Chrome on Windows uses D3D12 through Dawn.

## Goals / Non-Goals

**Goals:** like-for-like comparison (same scenes and settings, warm runs, medians), explicit targets.
**Non-Goals:** profiling inside the GPU (timestamp queries).

## Decisions

- **Same work both sides:** browser `profile=full` = native `upstream-basic`; browser `profile=dense` =
  native `upstream-basic-dense`. The first browser bake includes WGSL compilation and pipeline creation
  (reported separately as "first bake"); the median of the following N bakes (tile reuse off) is compared
  with the native median of 5.
- **Phase times in the browser** come from progress events (end of each phase's last batch), so "direct"
  includes rasterisation and "bounce 1" the CPU per-tile direct stage, as in the native breakdown.
- **Memory:** wasm linear memory (`WebAssembly.Memory.buffer.byteLength` after the bake) — the bridge exposes
  it; native peak working set from P1.5.
- **Targets:** derived from the measured dense bake on the Intel iGPU (the reference GPU here), with headroom
  for Phases 3–5 features and a Phase 6 optimisation goal.

## Risks / Trade-offs

- [One machine, one integrated GPU] → targets are per reference GPU; P6.8 adds the browser × GPU matrix.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none new (one stat read from wasm memory).
ADRs: none.
