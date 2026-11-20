# ADR-0008: Asset library — live polyfork.dev CDN (CC BY) plus GLB importer and primitives

- **Status:** accepted (re-verify terms at the start of Phase 5)
- **Date:** 2026-11-20
- **Phase:** 5
- **Requirements:** REQ-AST-01 … 05

## Context

The brief asks to populate scenes with polyfork.dev assets, verifying API and licensing first and
falling back to GLTF/GLB import. Verified facts are in [ASSET_LIBRARY.md §1](../ASSET_LIBRARY.md).

## Decision

- Live browser over polyfork's public REST API; GLBs loaded per use from `https://polyfork.dev/cdn/`
  (CORS open). Free tier only, GLB only.
- CC BY 4.0 licence path with automatic credits in the app and in exports.
- No persistent mirroring/caching of polyfork files by the app; scenes store ids/URLs.
- **Never commit polyfork files** to the repository (script-generated meshes; committing could be
  seen as redistribution). Linking the CDN is fine.
- Identification through the browser's `Referer`/`Origin` (no custom `User-Agent` possible).
- GLTF/GLB importer and built-in primitives are first-class (offline, tests, other sources).
- Per-vertex/per-triangle albedo support in the baker (polyfork assets are vertex-coloured).

## Alternatives considered

| Option | Why not |
|---|---|
| Use the MCP server from the app | Designed for agents; the REST API suffices for a browser UI. |
| Proxy/mirror assets through our own server | Needs polyfork's platform licence; adds infrastructure. |
| Use `.mjs` drop-in modules | Polyfork licence only (no redistribution); code execution from a third-party origin. |
| Commit a CC BY test set | Rejected by the project owner (redistribution concern). |

## Consequences

- The showcase needs network access to load polyfork assets; CI tests rely on primitives and our
  own fixtures and never touch the network.

## Verification

Phase 5: import of 20 diverse polyfork assets (geometry, vertex colours, scale) checked manually;
credits present in UI and export; 429 back-off tested with a mocked endpoint (Vitest).
