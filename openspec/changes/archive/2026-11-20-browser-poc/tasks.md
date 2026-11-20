# Tasks

## 1. Workspace and js

- [x] 1.1 Root `package.json` + `pnpm-workspace.yaml` (`js`, `app`); `js` gets Vitest and `readScene()`; test: reading `test-scenes/upstream-basic/upstream-basic.lbscene` gives 4 meshes / 538 triangles and `writeScene(readScene(x)) == x`; verify `pnpm -C js test`

## 2. Worker and protocol

- [x] 2.1 `app/` scaffold (Vite, strict TS, three, vitest, alias to the wasm pkg, `build:wasm`); verify `pnpm -C app typecheck` and `pnpm -C app build`
- [x] 2.2 `protocol.ts`, `bakerCore.ts`, `baker.worker.ts`, `client.ts`; Vitest with fake bridge/worker (progress, done with checks, cancelled, error kinds, concurrent bake, no-webgpu capabilities); verify `pnpm -C app test`

## 3. Page

- [x] 3.1 PoC page: capabilities/adapter panel, bake/cancel, progress, timings/transfers/warnings, checks, Three.js view with indirect / sun field / AO modes; verify in Chrome on the Intel GPU (headless via Playwright script): bake completes, 19 checks pass, cancel then rebake works; record adapter and time
- [x] 3.2 Update BUILD.md (pnpm, app commands verified), INTERFACES.md §7 (protocol additions), ARCHITECTURE.md §6 note, regenerate THIRD_PARTY_NOTICES (npm deps via extra entries); tick P2.5 in ROADMAP.md

## Workflow follow-up

- Archive (sync `browser-baking`), commit on `main`, push, confirm CI green.
