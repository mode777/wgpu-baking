# Proposal: phase-1-baseline (P1.5)

## Why

Phase 2 has to compare browser bakes against native ones (timings, readbacks, memory), and Phase 2's report
sets the performance targets for the rest of the project. That needs a recorded native baseline on a real
GPU and on lavapipe, and FEASIBILITY.md must be verified against the running code before we build on it
(Phase 1 exit criteria 1 and 3). Requirements: REQ-ENG-03, REQ-ENG-07, REQ-WASM-10; upstream behaviour
REQ-LGT-01, REQ-LGT-05, REQ-GI-01.

## What Changes

- Additive GPU transfer counters on `Gpu` (dispatch batches, readbacks and their bytes, uploaded bytes).
- Example `baseline`: bakes `upstream-basic` and a denser variant several times without cache and prints
  adapter, per-stage timings (ms), transfer counters, atlas/texel counts and peak memory as JSON + table.
- CI `rust-gpu` job runs the baseline example (informational) so lavapipe numbers are recorded.
- Report `docs/reports/phase-1-baseline.md`: adapters, timings, readbacks, memory, test results (hardware
  GPU locally, lavapipe in CI), Phase 1 exit-criteria evidence.
- FEASIBILITY.md corrected where the code differs.

## Capabilities

### New Capabilities

None (`skip_specs: true`): measurement tooling and documentation; no user-visible behaviour.

### Modified Capabilities

None.

## Out of scope

Performance targets (set in P2.8), any optimisation, browser numbers (Phase 2), upstream merge.

## Impact

`crates/lightbaker/src/gpu/mod.rs` (counters), new example, `.github/workflows/ci.yml`, `docs/reports/`,
`docs/FEASIBILITY.md`, ROADMAP current phase.
