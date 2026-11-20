## Why

P3.5 put xatlas behind the engine's unwrap contract natively. The browser runs xatlas as a separate
module (`xatlas-wasm`) in the baker worker; until now the worker unwrapped scenes in TypeScript, beside the
Rust pipeline. For P3.7 (object tiles packed by xatlas inside the bake) the Rust engine compiled to wasm has
to call xatlas through the same contract it uses natively, and both paths must be shown to give the same
UVs (ADR-0006, Roadmap P3.6).

## What Changes

- Engine: `lightbaker::unwrap::unwrap_scene` — one object tile per scene mesh through any `Unwrapper`, with
  the validity report of each mesh; used natively and in the browser.
- Bridge: `lightbaker-web` imports the JS adapter (`app/src/bake/xatlasUnwrapper.ts`, `unwrapTile`) through a
  `wasm-bindgen` extern type (`JsUnwrapper`), implements the `Unwrapper` contract over it, and exports
  `unwrapScene(scene, config, unwrapper)`; the worker's `unwrap` request now goes through it (the Rust UV
  checks replace the TS ones on that path).
- Equivalence fixture `test-scenes/uv-equivalence/` (scene + native unwrap written by an example): a native
  test and an app test (bridge + `xatlas-wasm` in Node) compare their unwraps with it.
- Browser e2e: the page's Unwrap goes through the bridge and every mesh is valid.

## Capabilities

### New Capabilities

### Modified Capabilities

- `lightmap-unwrap`: native/web equivalence; scene unwrap through the contract.
- `wasm-bridge`: unwrap through a JS unwrapper.

## Impact

`crates/lightbaker/src/unwrap.rs`, `crates/lightbaker-web/src/{lib.rs,unwrap.rs}`, `crates/lightbaker-uv`
(example, test), `app/src/bake/{unwrapScene,baker.worker}.ts`, app tests/e2e, `test-scenes/uv-equivalence/`,
docs (INTERFACES, ADR-0006, TESTING).
