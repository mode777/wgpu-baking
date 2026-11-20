# Proposal: browser-ci (P2.6)

## Why

S2 requires the upstream test scene to bake in Chrome on lavapipe **in CI** with metrics matching native, and
the capability message must keep working when WebGPU is missing. Today the browser path is only checked by
hand. ADR-0009 defines a `web` job; TESTING.md budgets the first two e2e specs. Requirements: REQ-WASM-04,
REQ-WASM-06, REQ-WASM-08, REQ-ENG-04, REQ-ENG-05.

## What Changes

- Playwright config in `app/` (`pnpm -C app e2e`) with two projects: `webgpu` (WebGPU flags from three.js'
  CI, headed under xvfb in CI) and `no-webgpu` (Chromium with WebGPU disabled), served from the production
  build via `vite preview`.
- `app/e2e/bake-poc.spec.ts`: bake `upstream-basic` with the CI profile in the worker, all `expect.json`
  checks pass, the adapter is shown (labelled when software), cancel mid-bake, then rebake passes.
- `app/e2e/no-webgpu.spec.ts`: the page explains that WebGPU is unavailable and Bake stays disabled.
- CI job `web` (ubuntu, `mesa-vulkan-drivers`, `xvfb`): pnpm install, `js` tests, wasm build, typecheck,
  Vitest, app build, Playwright e2e with `E2E_ALLOW_FALLBACK=1` (software adapter opt-in, labelled).
- The CI profile is confirmed to pass all checks on a software adapter.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci-pipeline`: adds the browser job and its e2e budget.
- `browser-baking`: the CI bake profile is defined as a requirement (checks must hold with it).

## Out of scope

`editor-smoke.spec.ts` (P5.9), device loss in the browser (P2.7, Vitest + manual), browser/GPU matrix
(Phase 6), image comparison.

## Impact

`app/` (Playwright config, specs, scripts), `.github/workflows/ci.yml`, TESTING.md, BUILD.md.
