# Tasks

## 1. Measurement tooling

- [x] 1.1 Page profile `dense`; bridge stat `wasmMemoryBytes`; `hw-check.mjs --bench N` (median per phase over N full bakes after the first); verify typecheck, Vitest, clippy for the bridge, and one run on the Intel GPU

## 2. Measurements

- [x] 2.1 Native: rerun `baseline` (Intel Vulkan and DX12) after Phase 2's engine changes; browser: Chrome and Edge on the Intel GPU, profiles `full` and `dense` (bench 5), SwiftShader `ci` profile; save JSON under `docs/reports/phase-2/`

## 3. Report

- [x] 3.1 Write `docs/reports/phase-2-webgpu.md` (exit criteria evidence, native vs browser, readbacks, limits, WGSL notes, memory, observations, performance targets); resolve Q4 in ROADMAP; ROADMAP current phase → 3, tick P2.8; README/AGENTS status; INTERFACES §8 stat

## Workflow follow-up

- Archive, commit on `main`, push, confirm CI green.
