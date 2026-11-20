# Proposal: xatlas-wasm-evaluation (P3.1)

## Why

ADR-0006 bets the browser unwrap on `xatlas-wasm` (pinned 0.2.0). Before scene format v2 and per-triangle UVs
are built on it, the package must be proven inside our baker worker under Vite: that it loads there, unwraps
our meshes per object tile with the options ADR-0004 needs (texels per metre, padding, material-separated
charts, auto-sized atlas) and what it costs (time, memory). Requirements: REQ-UV-01, REQ-UV-02, REQ-UV-05,
REQ-WASM-02, REQ-WASM-03.

## What Changes

- Dependency `xatlas-wasm@0.2.0` (exact) in `app`.
- `app/src/bake/xatlasUnwrapper.ts`: a thin adapter (`loadXatlas`, `unwrapTile`) returning typed arrays: UVs
  in tile texels, `xref`, new indices, chart ids, tile size, utilisation, timings.
- The worker implements the protocol's `unwrap` request (INTERFACES.md §7) for `.lbscene` files: each mesh
  is one object tile; the result is per-mesh UV data and statistics (`unwrapped`).
- PoC page: "Unwrap (xatlas)" button showing per-mesh charts, tile sizes, utilisation and time.
- Vitest (Node): UV validity on test meshes (no out-of-range UVs, no flipped triangles with area, `xref`
  reproduces positions, charts never span materials, density near the request) and timings/memory.
- ADR-0006 gains the evaluation results (browser and Node timings, memory, caveats).

## Capabilities

### New Capabilities

- `lightmap-unwrap`: automatic lightmap UVs per object tile and the validity guarantees they carry.

### Modified Capabilities

- `browser-baking`: the worker answers `unwrap` requests.

## Out of scope

Feeding the UVs into the bake (P3.3–P3.8), native xatlas (P3.5), the Rust `Unwrapper` contract wiring (P3.5/P3.6),
scene format v2 (P3.2).

## Impact

`app/` (dependency, adapter, worker, page, tests, eval script), INTERFACES.md §7, ADR-0006,
THIRD_PARTY_NOTICES.
