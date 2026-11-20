# Proposal: browser-poc (P2.5)

## Why

The bridge (P2.4) can bake from JavaScript, but there is no page that runs it in a browser. Phase 2's
deliverable is a browser proof of concept that bakes `upstream-basic` on WebGPU in a Web Worker, shows the
adapter, progress and timings, and displays the result (S2). It is also the page P2.6's e2e test drives.
Requirements: REQ-WASM-02 (worker), REQ-WASM-04 (WebGPU, adapter info), REQ-WASM-06 (progress, cancel),
REQ-WASM-08 (clear errors), REQ-ENG-05 (first scene in the browser).

## What Changes

- pnpm workspace (`js`, `app`); `app/` with Vite, strict TypeScript, Three.js and Vitest (no React yet).
- `app/src/bake/protocol.ts`: the worker protocol subset used now (INTERFACES.md §7: `init`, `bake`,
  `cancel`, `dispose` → `capabilities`, `progress`, `done`, `cancelled`, `error`).
- `app/src/bake/bakerCore.ts`: protocol handling over an injected bridge (testable without a browser);
  `baker.worker.ts` wires it to `lightbaker-web`; `client.ts` is the main-thread side (promises, events).
- PoC page: capability/adapter panel (software adapters labelled, `?allowFallbackAdapter=1` to opt in),
  bake / cancel / rebake, progress with phase and elapsed time, timings and transfer counts, the
  `expect.json` check results, warnings, and a Three.js view of the scene with the direct (sun field, AO)
  or indirect lightmap applied through the bake's chart UVs.
- `js/src/scene.ts`: `readScene()` (additive) so the page can draw the scene it bakes; Vitest for `js`.

## Capabilities

### New Capabilities

- `browser-baking`: baking in a dedicated worker from a page, the worker protocol, and what the page
  shows the user (adapter, progress, results, errors).

### Modified Capabilities

None.

## Out of scope

Editor UI and React (Phase 5), Playwright e2e and CI `web` job (P2.6), worker recreation after device loss
(P2.7), lighting-correct runtime shading (the view shows lightmaps, not a shaded scene).

## Impact

New `app/`, `pnpm-workspace.yaml`, root `package.json`, `js/` (reader + tests), `.gitignore`, BUILD.md,
THIRD_PARTY_NOTICES (npm deps), ARCHITECTURE/INTERFACES notes.
