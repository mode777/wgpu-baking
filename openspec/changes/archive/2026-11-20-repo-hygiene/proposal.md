# Proposal: repo-hygiene (P1.2)

## Why

After the fork (P1.1) the repository still lacks a pinned compiler, ignores for generated web output, a
third-party licence record and our copyright line. Reproducible builds (REQ-ENG-03) and the definition of
done ("licences of any new dependency recorded in `THIRD_PARTY_NOTICES.md`") depend on them. Decided with
the project owner (Q5): keep upstream's MIT text and Orbitoon line, add ours.

## What Changes

- `rust-toolchain.toml` pinning rustc `1.99.1` with `clippy`, `rustfmt` and the `wasm32-unknown-unknown` target.
- `.gitignore`: add `pkg/`, `dist/`, `node_modules/` (keeps upstream entries).
- `THIRD_PARTY_NOTICES.md`: upstream lightbaker (MIT), and the licences of every crate in `Cargo.lock`.
- `LICENSE`: add `Copyright (c) 2026 The wgpu-baking contributors` below the unchanged Orbitoon line.
- Workspace `repository` field points to this fork; BUILD.md toolchain section updated.

## Capabilities

### New Capabilities

None (`skip_specs: true`): repository metadata only, no system behaviour.

### Modified Capabilities

None.

## Out of scope

CI (P1.3), any source change, JS tooling (pnpm workspace arrives in P2).

## Impact

Root files only. `cargo` will auto-install the pinned toolchain on first use.
