# Design: repo-hygiene

## Context

Upstream has no `rust-toolchain.toml`; its `.gitignore` covers `/target`, `/js/node_modules`, `*.lbake`,
`.lbcache/`. BUILD.md requires Rust ≥ 1.87 (wgpu 30 MSRV).

## Goals / Non-Goals

**Goals:** one compiler version for local, CI and later wasm builds; a licence inventory that is easy to
regenerate. **Non-Goals:** licence automation in CI (can be added later, e.g. `cargo deny`).

## Decisions

- **Pin an exact version (`1.99.1`), not `stable`.** `stable` moves and breaks determinism of clippy
  lints and codegen across CI runs. 1.99.1 is the version verified locally.
- **Targets in the toolchain file** (`wasm32-unknown-unknown`) so every developer and CI job gets the web
  target without a separate `rustup target add`.
- **Notices generated from `cargo metadata`** (name, version, licence expression), grouped by licence, plus
  upstream lightbaker. The generation command is documented at the top of the file so it can be rerun when
  `Cargo.lock` changes.
- **LICENSE edit is a single added line**, keeping upstream text byte-identical otherwise (Q5, ADR-0001).

## Risks / Trade-offs

- [Pinned toolchain becomes stale] → bump deliberately in a dedicated change.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none. ADRs: none (implements ADR-0001).
