# Design: browser-ci

## Context

The PoC page exposes `data-testid`s and `body[data-state]` (`ready`, `baking`, `done`, `cancelled`,
`checks-failed`, `error-*`, `no-webgpu`, `fallback-adapter-refused`). `app/scripts/hw-check.mjs` already
proved the flow in Chrome on the Intel GPU and on SwiftShader (CI profile, 19/19 checks).

## Goals / Non-Goals

**Goals:** two cheap, robust specs (< 2 min on lavapipe); same specs runnable locally on a hardware GPU.
**Non-Goals:** timing assertions (software adapters), screenshots/image diff.

## Decisions

- **Projects:** `webgpu` → Chromium with `--enable-unsafe-webgpu --enable-features=Vulkan
  --disable-vulkan-surface --ignore-gpu-blocklist --disable-gpu-watchdog` (three.js' CI set; `--no-sandbox`
  only on Linux CI); `no-webgpu` → `--disable-blink-features=WebGPU`, which removes `navigator.gpu` in the
  page and in workers (a real absence, no test hooks in the app).
- **Browser:** Playwright's Chromium by default; `E2E_CHANNEL=chrome|msedge` uses an installed browser
  (local hardware runs). `E2E_HEADED=1` (CI under `xvfb-run`): headed Chromium is what three.js' CI uses
  for WebGPU on lavapipe.
- **Software adapter opt-in by environment:** `E2E_ALLOW_FALLBACK=1` adds `allowFallbackAdapter=1`; the spec
  then expects the SOFTWARE ADAPTER label; without it, the spec expects no label (hardware run).
- **Metrics vs expect.json:** the spec asserts `checks-summary[data-passed=true]` and 19 checks (the
  worker evaluates `expect.json` in Rust, the same code as native). Reference values are not compared in
  CI (rays differ in the CI profile; adapters differ); native-vs-browser parity on the full profile is
  recorded in the Phase 2 report.
- **Cancel:** with "reuse baked tiles" off, click Bake, wait for a progress phase, click Cancel; the outcome
  is `cancelled` (or `done` if the bake already finished — accepted and logged, as on fast GPUs the
  window is short); then rebake must end `done` with all checks passing.
- **web job:** `pnpm/action-setup` (version from `packageManager`), `actions/setup-node` 22 with pnpm cache,
  wasm-pack via `taiki-e/install-action`, `playwright install --with-deps chromium`, artefacts:
  Playwright report on failure.

## Risks / Trade-offs

- [Chromium on lavapipe flaky or slow] → generous per-test timeout (3 min), one retry in CI, CI profile.
- [`--disable-blink-features=WebGPU` stops working] → the spec asserts the message text, so a change would
  fail loudly.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none. ADRs: none (implements ADR-0009).
