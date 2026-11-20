# Tasks

## 1. Specs

- [x] 1.1 `app/playwright.config.ts` (projects `webgpu`/`no-webgpu`, webServer `vite preview`, env switches) and `pnpm -C app e2e`; verify `pnpm -C app exec playwright test --list` shows exactly two spec files
- [x] 1.2 `app/e2e/bake-poc.spec.ts` and `app/e2e/no-webgpu.spec.ts`; verify locally on the Intel GPU (`E2E_CHANNEL=chrome`) and on SwiftShader with `E2E_ALLOW_FALLBACK=1` (software path)

## 2. CI

- [x] 2.1 Add the `web` job to `.github/workflows/ci.yml`; verify on GitHub that both specs pass on lavapipe and the log shows the software adapter label

## 3. Docs

- [x] 3.1 Update TESTING.md (e2e budget status, env switches) and BUILD.md (`e2e` command verified, CI table); tick P2.6 in ROADMAP.md

## Workflow follow-up

- Archive (sync `ci-pipeline`, `browser-baking`), commit on `main`, push.
