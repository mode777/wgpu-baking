# ADR-0001: Fork strategy and repository layout

- **Status:** accepted
- **Date:** 2026-11-20
- **Phase:** 1
- **Requirements:** REQ-ENG-07, REQ-WASM-10, REQ-ENG-03

## Context

The project extends [jer-nc/lightbaker](https://github.com/jer-nc/lightbaker) (MIT, inspected at
commit `4fe85da`, 2026-11-19). Upstream is young and active ("formats, settings and API may still
change"). We want its future fixes, and the brief asks to preserve its architecture.

## Decision

- Import upstream with full history; add it as remote `upstream`. Our work lives on `main`.
- Keep upstream paths unchanged: `crates/lightbaker`, `crates/lightbaker-cli`, `js/`, `docs/DESIGN.md`,
  `docs/FORMAT.md`. Upstream's `README.md` moves to `docs/UPSTREAM_README.md`; our `README.md`
  replaces it.
- New code goes into new crates/packages (`crates/lightbaker-uv`, `crates/lightbaker-web`, `app/`,
  `test-scenes/`). Changes inside upstream files are kept minimal, additive and feature-gated
  (`native`/`web`) where possible.
- Keep upstream's MIT `LICENSE` text and its `Copyright (c) 2026 Orbitoon` line unchanged and add
  `Copyright (c) 2026 The wgpu-baking contributors` below it (decided 2026-11-20); track
  third-party licences in `THIRD_PARTY_NOTICES.md`.
- Merge upstream periodically (at least at each phase boundary); conflicts are resolved in a
  dedicated change.

## Alternatives considered

| Option | Why not |
|---|---|
| Vendor upstream as a git submodule and wrap it | Many changes must go inside upstream modules (async GPU, per-triangle UVs, new passes); a submodule would force a fork anyway. |
| Rewrite with lightbaker as "inspiration" | Violates the brief (preserve architecture), loses tested behaviour. |

## Consequences

- Upstream style (long lines, prose comments, no rustfmt config) is kept in upstream files; we do
  not reformat them.
- Format versions (`.lbscene`, `.lbake`) are extended compatibly (v2 alongside v1).

## Verification

Phase 1 exit: upstream tests pass on the fork natively; `git merge upstream/main` is a no-op.
