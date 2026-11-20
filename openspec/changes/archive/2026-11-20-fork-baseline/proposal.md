# Proposal: fork-baseline (P1.1)

## Why

This folder holds only planning documents. Every later roadmap task changes upstream lightbaker code, so the
fork must first exist with upstream's full history and an `upstream` remote, so that upstream fixes stay
mergeable (ADR-0001). Requirements: REQ-ENG-07 (preserve upstream architecture), REQ-WASM-10 (native build
kept), REQ-ENG-03.

## What Changes

- Initialise a git repository on `main` and commit the existing planning documents.
- Add the remote `upstream` = `https://github.com/jer-nc/lightbaker` and merge `upstream/main` (commit
  `4fe85da`) with `--allow-unrelated-histories`.
- Move upstream's `README.md` to `docs/UPSTREAM_README.md`; our `README.md` stays the project README.
- Keep every other upstream path unchanged (`crates/`, `js/`, `docs/DESIGN.md`, `docs/FORMAT.md`, `LICENSE`, …).
- Add the public GitHub repository `mode777/wgpu-baking` as `origin` and push `main`.
- Update README/AGENTS status lines and tick P1.1 in ROADMAP.md.

## Capabilities

### New Capabilities

None. This change only imports code and history; no system behaviour changes (`skip_specs: true`).

### Modified Capabilities

None.

## Out of scope

- Toolchain pinning, `.gitignore`, `THIRD_PARTY_NOTICES.md`, LICENSE copyright line (P1.2).
- CI workflows (P1.3), test scenes (P1.4), baseline measurements (P1.5).
- Any change to upstream source code.

## Impact

- New: `.git/`, upstream sources (`crates/lightbaker`, `crates/lightbaker-cli`, `js/`, `Cargo.toml`,
  `Cargo.lock`, `LICENSE`, upstream docs), `docs/UPSTREAM_README.md`.
- Remotes: `upstream` (jer-nc/lightbaker), `origin` (mode777/wgpu-baking, public).
