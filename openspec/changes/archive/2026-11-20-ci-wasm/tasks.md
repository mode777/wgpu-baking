# Tasks

## 1. Workflow

- [x] 1.1 Add the `wasm` job to `.github/workflows/ci.yml` (wasm32 build, web-feature unit tests, dependency guard); verify locally that each step's command passes and that the guard fails when run against the native feature set (`--features native` lists rayon)
- [x] 1.2 Push and verify the `wasm` job passes on GitHub

## 2. Docs

- [x] 2.1 Update BUILD.md CI section (`wasm` job status, guard); tick P2.3 in ROADMAP.md

## Workflow follow-up

- Archive (sync `ci-pipeline`), commit on `main`, push.
