# Build instructions

Status: **planned**. These instructions describe the target setup; each command is verified (and
this file corrected) in the phase that introduces it. Commands marked *(P1)*, *(P2)*, … become valid
at that phase.

## Toolchain

| Tool | Version | Purpose | Notes |
|---|---|---|---|
| Rust (rustup) | stable ≥ 1.87 (wgpu 30 MSRV; edition 2024 needs ≥ 1.85) | engine, CLI, bridge | pinned via `rust-toolchain.toml` (P1) |
| `wasm32-unknown-unknown` target | — | browser build | `rustup target add wasm32-unknown-unknown` |
| `wasm-bindgen-cli` / `wasm-pack` | matching the `wasm-bindgen` crate version | JS bindings | version pinned in BUILD.md at P2 |
| C++ compiler (MSVC / clang / gcc) | — | native xatlas (`cc`) | P3; the web path uses the prebuilt `xatlas-wasm` npm package (no Emscripten/clang needed) |
| Node.js | ≥ 22 LTS | app, tests | |
| pnpm | ≥ 9 (via `corepack enable`) | JS workspace | |
| Chromium-based browser | Chrome/Edge ≥ 113 | WebGPU baking | see RESEARCH.md §4 |
| Playwright browsers | pinned by the app | e2e | `pnpm -C app exec playwright install chromium` |

Environment observed when this plan was written (Windows): rustc 1.99.1, cargo 1.99.1, Node
26.13.0, npm 11.20.0; installed Rust targets: `x86_64-pc-windows-msvc` only; pnpm, wasm-pack and
wasm-bindgen-cli not installed.

## One-time setup

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <pinned>      # or: cargo install wasm-pack
corepack enable
pnpm install                                           # (P2) at the repository root
```

## Native engine

```powershell
cargo build --release -p lightbaker-cli                                    # (P1)
cargo run --release -p lightbaker-cli -- scene.lbscene --out baked/ --cache .lbcache/   # (P1, upstream usage)
cargo test -p lightbaker                                                   # (P1) GPU tests skip without an adapter
```

## WASM engine

```powershell
cargo build -p lightbaker --target wasm32-unknown-unknown --no-default-features --features web   # (P2)
wasm-pack build crates/lightbaker-web --target web --release                                     # (P2) → crates/lightbaker-web/pkg
cargo test -p lightbaker-web --target wasm32-unknown-unknown                                     # (P2) wasm-bindgen-test runner
```

## App

```powershell
pnpm -C app dev          # (P2) Vite dev server; open the printed URL in Chrome/Edge
pnpm -C app build        # (P2) production build in app/dist
pnpm -C app test         # (P2) Vitest
pnpm -C app e2e          # (P2) small Playwright set (Chrome with WebGPU enabled), see TESTING.md
```

WebGPU in automated Chrome needs flags. The Playwright config (P2) uses the set proven in three.js'
CI: `--enable-unsafe-webgpu --enable-features=Vulkan --disable-vulkan-surface
--ignore-gpu-blocklist --disable-gpu-watchdog --no-sandbox`, headed under `xvfb-run` on Linux. On a
machine without a hardware GPU this uses a software adapter, which the app only accepts with
`allowFallbackAdapter: true`; results are labelled.

## Running GPU tests locally on a software adapter (Linux)

```bash
sudo apt-get install -y mesa-vulkan-drivers xvfb          # lavapipe (software Vulkan)
LIGHTBAKER_REQUIRE_GPU=1 LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1 cargo test -p lightbaker
xvfb-run -a pnpm -C app e2e
```

## Reproducibility

- `Cargo.lock` and `pnpm-lock.yaml` are committed; `rust-toolchain.toml` pins the compiler.
- `wasm-bindgen` crate and CLI versions must match exactly (pinned together).
- xatlas: native vendors `jpcy/xatlas` at `f700c77` with its licence
  (`crates/lightbaker-uv/vendor/xatlas`); the web uses `xatlas-wasm` pinned to an exact version
  built from the same commit (ADR-0006).

## Continuous integration (GitHub Actions, ADR-0009)

Standard GitHub-hosted runners only; GPU work runs on software adapters (Mesa lavapipe).

| Job | Runner | Steps |
|---|---|---|
| `rust-native` | ubuntu-latest, windows-latest | fmt/clippy (new crates), build, unit tests |
| `rust-gpu` | ubuntu-latest | `apt-get install mesa-vulkan-drivers`; GPU integration tests with `LIGHTBAKER_REQUIRE_GPU=1`, `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1` |
| `wasm` | ubuntu-latest | wasm32 build, `wasm-pack build`, `wasm-bindgen-test` (Node, non-GPU) |
| `web` | ubuntu-latest | `mesa-vulkan-drivers xvfb`; pnpm install, typecheck, Vitest, app build, `xvfb-run -a pnpm -C app e2e` |
| `gpu-perf` (optional) | GPU larger runner / self-hosted, `workflow_dispatch` | real-GPU timings for phase reports |

Each job should stay under ~10 minutes. Performance numbers from software adapters are not
meaningful; real-GPU measurements go into the phase reports.
