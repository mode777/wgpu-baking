# Tasks

## 1. Features and shims

- [x] 1.1 Workspace `wgpu` without default features; `lightbaker` features `native` (default) / `web`; optional rayon, pollster, zstd; add `web-time`; `required-features = ["native"]` for GPU tests/examples; verify `cargo build --workspace` passes
- [x] 1.2 Add `par.rs` shim and switch `bake.rs`/`deps.rs` imports; `web-time` in `bake.rs`/`volume.rs`; verify native tests pass and `.lbake` of `upstream-basic` is byte-identical to before (hash recorded before the change)
- [x] 1.3 `TileStore` trait, `FsStore` (native), `MemoryStore`; `Options::cache: Option<&dyn TileStore>`; update CLI, tests, examples; add GPU test: second bake with the same `MemoryStore` bakes 0 tiles and gives identical output; verify `cargo test -p lightbaker --release`
- [x] 1.4 KTX2 supercompression by feature; unit test for scheme 0 under `web` and scheme 2 under `native`; verify `cargo test -p lightbaker --no-default-features --features web --lib` and default `--lib`
- [x] 1.5 Gate `Gpu::new`/pollster to `native`; verify `cargo build -p lightbaker --target wasm32-unknown-unknown --no-default-features --features web` succeeds and `cargo build -p lightbaker-cli` still works

## 2. Docs

- [x] 2.1 Update ADR-0003 (accepted; decisions 5–6 as implemented), BUILD.md (feature flags, wasm32 command verified), FEASIBILITY.md B4–B7 status, regenerate THIRD_PARTY_NOTICES.md; tick P2.1 in ROADMAP.md

## Workflow follow-up

- Archive (sync `engine-platforms`), commit on `main`, push, confirm CI green.
