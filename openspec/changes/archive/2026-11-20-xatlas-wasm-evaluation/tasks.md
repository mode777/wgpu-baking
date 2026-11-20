# Tasks

## 1. Adapter and checks

- [x] 1.1 Add `xatlas-wasm@0.2.0` (exact); `xatlasUnwrapper.ts` (`loadXatlas`, `unwrapTile`) and `uvValidity.ts`; Vitest in Node: unit quad size, multi-material charts, validity + timings on upstream-basic meshes, sphere, torus knot; verify `pnpm -C app test`

## 2. Worker and page

- [x] 2.1 Worker `unwrap` request → `unwrapped`; client `unwrap()`; Vitest with a fake unwrapper; page button and stats; verify typecheck/build and e2e unaffected
- [x] 2.2 `app/scripts/hw-check.mjs --unwrap`: unwrap in Chrome on the Intel GPU machine (cold and warm), record load time and per-mesh times

## 3. Docs

- [x] 3.1 ADR-0006 evaluation section (Node + browser numbers, memory, caveats), INTERFACES.md §7 (`unwrap`/`unwrapped` as implemented), THIRD_PARTY_NOTICES (xatlas-wasm, xatlas); tick P3.1 in ROADMAP.md

## Workflow follow-up

- Archive (sync `lightmap-unwrap`, `browser-baking`), commit on `main`, push, confirm CI green.
