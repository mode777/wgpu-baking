# Tasks

## 1. Engine

- [x] 1.1 Rust test: cancel during bounce 1 with a `MemoryStore` → `Cancelled`, store empty, next bake bakes all tiles with identical bytes; verify on the Intel GPU

## 2. Worker and page

- [x] 2.1 Core: `debug-lose-device` request, single global device-lost report per baker; `BakerService` with worker factory, states and `recreate()`; Vitest: loss outside and during a bake, recreate gives a ready baker that bakes; verify `pnpm -C app test`
- [x] 2.2 Page: device-lost message and "Recreate baker" button, `?debug=1` "Lose device" button; verify `pnpm -C app typecheck build` and the e2e specs still pass locally
- [x] 2.3 `hw-check.mjs --lose-device`: real `device.destroy()` on the Intel GPU in Chrome → loss shown → recreate → bake passes; record the output for the Phase 2 report

## 3. Docs

- [x] 3.1 Update TESTING.md (device loss coverage), INTERFACES.md §7 (`debug-lose-device`, loss reporting), ADR-0002 status; tick P2.7 in ROADMAP.md

## Workflow follow-up

- Archive (sync `browser-baking`, `bake-control`), commit on `main`, push, confirm CI green.
