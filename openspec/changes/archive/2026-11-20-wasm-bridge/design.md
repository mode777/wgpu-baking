# Design: wasm-bridge

## Context

`Gpu::open/run` and `bake` are async (P2.2). `Options { cache, log }`; `log: &(dyn Fn(&str) + Sync)` is only
called sequentially. JS callbacks (`js_sys::Function`) are neither `Send` nor `Sync`. wgpu's default uncaptured
error handler panics, which aborts a wasm module. wasm-bindgen async methods cannot borrow `self` across awaits.

## Goals / Non-Goals

**Goals:** a small, typed JS API sufficient for the PoC and e2e; engine hooks usable from JS; no panics on GPU
errors. **Non-Goals:** the worker protocol itself (TS, P2.5), result paging (Phase 3).

## Decisions

- **Hooks without `Sync`.** `Options { progress: Option<&dyn Fn(Progress)>, cancel: Option<&dyn Fn() -> bool> }`
  — called only from the sequential bake/run code, so JS-backed closures work. `Options: Default` (no-op log)
  so call sites write `..Default::default()`. `Gpu::run` takes `&Hooks` + the phase; it checks `cancel` before
  each batch and after each readback and reports progress after each batch. `bake` also checks before each
  stage. `Cancelled` is a unit error type (downcastable).
- **Platform selection by `target_arch`.** Features unify across the workspace (`lightbaker-web` asks for `web`,
  the CLI for `native`); `cfg(target_arch = "wasm32")` decides the instance backends and whether to poll.
- **GPU errors:** `Gpu::open` installs `on_uncaptured_error` storing the first error in a `Mutex<Option<String>>`
  and `set_device_lost_callback` storing the loss reason and forwarding to an optional user callback. `run`
  checks both after each await and returns `GpuError::DeviceLost`/`GpuError::Gpu`. Natively this turns former
  panics into errors (better), with no change in successful bakes.
- **`Gpu::probe()`**: adapter report from `request_adapter` without `request_device`, for `capabilities()`.
- **Bridge state:** `WebBaker { inner: Rc<Inner> }`, `Inner { gpu, store: MemoryStore, cancel: Cell<bool>,
  busy: Cell<bool>, lost: RefCell<Option<Function>>, last: RefCell<Option<(Scene, Bake)>> }`. `bake` returns a
  `Promise` from `future_to_promise(async move { … })` holding an `Rc` clone, so `cancel()` can run meanwhile.
- **`navigator.gpu` check** via `js_sys::Reflect` on the global before creating a wgpu instance (wgpu's webgpu
  instance creation without `navigator.gpu` is not a recoverable error path).
- **Errors to JS:** plain objects `{ kind, message }` built with `serde-wasm-bindgen`; mapping in a pure Rust
  `errors.rs` (`GpuError` → kinds, `Cancelled` → `cancelled`, scene read errors → `invalid-scene`), unit-tested
  on the host.
- **Results:** metadata via `serde-wasm-bindgen`; arrays as `Uint8Array`/`Float32Array` copies set with
  `Reflect::set` (the worker transfers them onward). The `.lbake` uses KTX2 without zstd on the web (P2.1).
- **Config overrides** in a pure Rust `config.rs` (serde, camelCase, partial): applied to `SceneHeader`; unknown
  or not-yet-supported fields collected as warnings, never silently dropped.
- **Testing:** host `cargo test -p lightbaker-web` for `config`/`errors`; `wasm-pack test --node` for
  `capabilities()` and `create()` without WebGPU; real GPU paths in P2.5/P2.6. CI installs wasm-pack with
  `taiki-e/install-action` (prebuilt), which fetches the matching `wasm-bindgen` CLI.

## Risks / Trade-offs

- [`Rc`/`Cell` state while a JS callback re-enters the baker] → `busy` flag rejects concurrent bakes; `cancel`
  only sets a flag.
- [Copying result arrays doubles peak memory briefly] → small at PoC sizes; Phase 3 pages results.

GPU-to-CPU readbacks: none new (results are already on the CPU after the bake). Blocking calls: none on the web
path. Native-only dependencies: none (wasm-bindgen family builds for both). ADRs: none new; ADR-0002/0003
referenced; INTERFACES.md §7–9 updated with the implemented API.
