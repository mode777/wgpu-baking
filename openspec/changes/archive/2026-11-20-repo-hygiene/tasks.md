# Tasks

## 1. Repository files

- [x] 1.1 Add `rust-toolchain.toml` (1.99.1, clippy, rustfmt, wasm32 target); verify `rustup show active-toolchain` reports it and `cargo build -p lightbaker-cli` succeeds
- [x] 1.2 Extend `.gitignore` with `pkg/`, `dist/`, `node_modules/`; verify `git check-ignore app/node_modules/x crates/lightbaker-web/pkg/x app/dist/x` matches all three
- [x] 1.3 Add `Copyright (c) 2026 The wgpu-baking contributors` to LICENSE; verify `git diff upstream/main -- LICENSE` shows exactly one added line
- [x] 1.4 Generate `THIRD_PARTY_NOTICES.md` from `cargo metadata` (all locked crates + upstream lightbaker); verify every package in `Cargo.lock` is listed
- [x] 1.5 Point workspace `repository` to `https://github.com/mode777/wgpu-baking`; verify `cargo metadata` succeeds

## 2. Docs

- [x] 2.1 Update BUILD.md toolchain table/observed environment (pinned toolchain, pnpm/wasm-pack installed); tick P1.2 in ROADMAP.md

## Workflow follow-up

- Archive, commit on `main`, push.
