# Tasks

## 1. Engine

- [x] 1.1 Add `futures-channel`; `GpuOptions`, `GpuError`, `AdapterReport`, `admit()`, `required_limits()`; web instance with `BROWSER_WEBGPU`; unit tests for `admit` and `required_limits`; verify `cargo test -p lightbaker --lib`
- [x] 1.2 Async `Gpu::run` with oneshot readback (poll only under `native`), size checks against granted limits returning `LimitExceeded`; unit test for the size check; verify native GPU tests pass
- [x] 1.3 `bake()` and `bake_volume()` async; update CLI (`--allow-software-adapter`, adapter report), tests (helper passes the opt-in), examples with `pollster::block_on`; verify `cargo test -p lightbaker --release` on the Intel GPU and that `upstream-basic.lbake` bytes are unchanged
- [x] 1.4 Verify `cargo build -p lightbaker --target wasm32-unknown-unknown --no-default-features --features web` and that the web build contains no `pollster`/`wait_indefinitely` (`cargo tree` / cfg review)

## 2. Docs

- [x] 2.1 Update ADR-0003 (points 2, 7, 8 implemented), INTERFACES.md §9 (actual signatures), BUILD.md (CLI flag), FEASIBILITY.md (B1–B3, B8 fixed), regenerate THIRD_PARTY_NOTICES.md; tick P2.2 in ROADMAP.md

## Workflow follow-up

- Archive (sync `gpu-access`), commit on `main`, push, confirm CI green (lavapipe exercises the opt-in path).
