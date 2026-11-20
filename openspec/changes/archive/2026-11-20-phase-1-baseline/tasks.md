# Tasks

## 1. Instrumentation

- [x] 1.1 Add transfer counters to `Gpu` (`Transfers { batches, readbacks, readback_bytes, upload_bytes }`, `transfers()`, `reset_transfers()`); verify a unit/integration assertion that a bake of `upstream-basic` reports readbacks == batches > 0
- [x] 1.2 Add example `baseline` (upstream-basic + dense variant, warm-up + 5 runs, median per stage, counters, VmHWM on Linux, JSON + table); verify it runs on the Intel GPU
- [x] 1.3 Add the baseline example as an informational step to the CI `rust-gpu` job; verify on GitHub it prints lavapipe numbers

## 2. Report

- [x] 2.1 Run `cargo test -p lightbaker --release` with `LIGHTBAKER_REQUIRE_GPU=1` on the Intel GPU and record adapter, results, timings, readbacks, peak memory; collect the lavapipe run from CI
- [x] 2.2 Verify FEASIBILITY.md §2–§5 (sizes, versions, blockers, bindings, batch sizes, struct sizes) against the code; correct differences
- [x] 2.3 Write `docs/reports/phase-1-baseline.md` with exit-criteria evidence; update BUILD.md (baseline command), ROADMAP (tick P1.5, current phase → 2), AGENTS/README status

## Workflow follow-up

- Archive, commit on `main`, push, confirm CI green.
