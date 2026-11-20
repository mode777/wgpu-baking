# Design: phase-1-baseline

## Context

`bake()` logs stages through `Options::log` with 0.1 s resolution; `Gpu::run` submits one dispatch per
batch and reads each back (map_async + poll); the `light` atlas is uploaded on every `run` call. The
upstream scene bakes in ~0.1 s on the local Intel GPU, too small to time stages meaningfully.

## Goals / Non-Goals

**Goals:** reproducible numbers with a documented command; transfer counts that Phase 2 can compare.
**Non-Goals:** profiling GPU kernels (timestamp queries), changing upstream log output.

## Decisions

- **Stage timing from log timestamps.** The example's `log` callback records `Instant` at each line; a
  stage's time is the gap to the previous line. No change to `bake.rs`. Alternative: add timers inside
  `bake()` — rejected (upstream diff, and P2.1 replaces `Instant` anyway).
- **Counters as `AtomicU64` fields on `Gpu`** with `Gpu::transfers()` returning a `Transfers` snapshot and
  `reset_transfers()`. Atomics keep `Gpu: Sync` (it is shared across rayon closures). They cost a relaxed
  add per batch.
- **Dense variant** = `upstream-basic` with `texelsPerMeter` 16 (×4), `atlasWidth` 2048, `giRays` 512,
  `giBounces` 3, mutated in memory (not committed): enough work to see per-stage GPU time.
- **Repetitions:** 1 warm-up + 5 measured bakes; report the median per stage.
- **Peak memory:** CPU peak working set (`/proc/self/status` VmHWM on Linux; on Windows measured externally
  with `Get-Process` PeakWorkingSet64). GPU memory is estimated from buffer sizes (BVH, triangles, texels
  64 B, light atlas 16 B/texel), since wgpu exposes no portable allocation report.

## Risks / Trade-offs

- [lavapipe timings vary across runners] → labelled informational; only the real-GPU numbers feed P2.8.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none new (counting existing readbacks only).
ADRs: none created or updated.
