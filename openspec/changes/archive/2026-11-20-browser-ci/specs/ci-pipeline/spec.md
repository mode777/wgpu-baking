# Spec Delta

## ADDED Requirements

### Requirement: CI bakes in a browser
CI SHALL build the web app and run the browser end-to-end specs in Chromium on a Mesa lavapipe software
adapter (explicitly allowed and labelled) on every push and pull request, within the e2e budget of
docs/TESTING.md (at most three specs).

#### Scenario: Browser bake in CI
- **WHEN** the `web` job runs
- **THEN** `bake-poc.spec.ts` bakes `upstream-basic` with the CI profile in the worker and all its checks pass

#### Scenario: Missing WebGPU in CI
- **WHEN** the `no-webgpu` project runs
- **THEN** `no-webgpu.spec.ts` sees the capability message and a disabled Bake button
