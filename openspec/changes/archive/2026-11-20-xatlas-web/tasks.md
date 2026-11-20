# Tasks

## 1. Engine and native

- [x] 1.1 `lightbaker::unwrap::unwrap_scene` (+ unit test with a fake unwrapper)
- [x] 1.2 Example `export-uv-equivalence` (lightbaker-uv) writing `test-scenes/uv-equivalence/`; native test comparing with it; verify `cargo test -p lightbaker-uv`

## 2. Bridge and worker

- [x] 2.1 `lightbaker-web`: `JsUnwrapper` extern type, `Unwrapper` impl, `unwrapScene` export, typed errors; Node wasm-bindgen test with a fake unwrapper; clippy (wasm32)
- [x] 2.2 Worker `unwrap` through the bridge (`unwrapScene.ts`); app test: equivalence fixture through bridge + `xatlas-wasm` (Node, `initSync`); verify typecheck + Vitest
- [x] 2.3 e2e: Unwrap on the page, all meshes valid; verify Playwright locally (Chrome)

## 3. Docs

- [x] 3.1 INTERFACES (bridge `unwrapScene`), ADR-0006 (P3.6 section with measured tolerance), TESTING (equivalence) (test-scenes are documented there); tick P3.6 in ROADMAP.md
