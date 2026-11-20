# Proposal: upstream-test-scene (P1.4)

## Why

Upstream's only end-to-end scene lives inside `tests/bake.rs` as code, so neither the CLI nor (in Phase 2)
the browser can bake it, and its assertions cannot be reused. Phase 1 exit criterion 2 needs the CLI to
bake `test-scenes/upstream-basic` reproducibly, and Phase 2 needs the same scene and metrics in the browser
(TESTING.md). Requirements: REQ-ENG-05 (deterministic test scenes), REQ-ENG-04, REQ-LGT-01, REQ-LGT-05,
REQ-GI-01 (upstream behaviour pinned by metrics).

## What Changes

- The upstream scene builder moves into a shared file used by `tests/bake.rs` (unchanged assertions) and a
  new example `cargo run -p lightbaker --example export-upstream-basic` that writes
  `test-scenes/upstream-basic/upstream-basic.lbscene` and `expect.json`.
- `expect.json` format (documented in TESTING.md): named world-space probes on the direct, indirect or volume
  output, and checks comparing probe components (the upstream assertions: shadow, sun, foot occlusion, red
  bleed ratio, sky light, volume sun visibility, strip coverage), plus native reference values.
- New additive module `lightbaker::expect`: parse `expect.json`, sample a bake at probe points, evaluate checks.
- CLI flag `--check`: evaluates `expect.json` next to each scene and fails on a violated check.
- Integration test: the committed scene equals the generator's output; baking it twice gives identical
  `.lbake` bytes; all checks pass.

## Capabilities

### New Capabilities

- `test-scenes`: committed deterministic scenes with machine-checkable expectations, and how bakes are checked against them.

### Modified Capabilities

None.

## Out of scope

Other deterministic scenes (Phases 3–4), CI-sized variant/bake profile (P2.6), browser evaluation (P2.4–P2.6),
timings (P1.5).

## Impact

`crates/lightbaker` (new `expect` module, example, tests), `crates/lightbaker-cli` (`--check`), new
`test-scenes/upstream-basic/`, TESTING.md, BUILD.md.
