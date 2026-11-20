# Proposal: phase-2-report (P2.8)

## Why

Phase 2 ends with a report: how the browser bake compares with native (timings, readbacks, limits, WGSL
compilation, memory), the hard-gate evidence (S1, S2 on a hardware adapter, cancel and device loss), and the
**performance targets** for Phases 3–6 (open question Q4), set from measurements rather than guessed
(ROADMAP rules). Requirements: REQ-WASM-01 … 04, -06 … -08, -10; REQ-ENG-05.

## What Changes

- Page profile `dense` (the P1.5 dense variant: 16 texels/m, 2048 atlas, 512 rays, 3 bounces) so native and
  browser bake the same work.
- `hw-check.mjs --bench N`: N full bakes (no tile reuse) after the first, median per phase; records the
  bridge's wasm memory size.
- Bridge stats gain `wasmMemoryBytes` (the wasm heap after the bake).
- Report `docs/reports/phase-2-webgpu.md` with raw JSON in `docs/reports/phase-2/`; performance targets
  recorded there and in ROADMAP (Q4 resolved); ROADMAP current phase → 3; README/AGENTS status.

## Capabilities

### New Capabilities

None (`skip_specs: true`): measurement tooling and documentation.

### Modified Capabilities

None.

## Out of scope

Optimisation (Phase 6), browsers other than Chromium-based ones on this machine (P6.8 matrix), real-GPU CI.

## Impact

`app/src/main.ts` (profile), `app/scripts/hw-check.mjs`, `crates/lightbaker-web/src/lib.rs` (one stat),
`docs/reports/`, ROADMAP.md, README.md, AGENTS.md, INTERFACES.md §8 (stat).
