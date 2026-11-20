# wgpu-baking — browser-based global illumination baker

A browser application for building small 3D scenes and **baking high-quality global illumination
into lightmaps directly in the browser on the GPU** — multi-bounce diffuse GI, colour bleeding,
soft shadows, emissive surfaces, point/spot/area lights — with automatic lightmap UVs.

Visual target: stylized, fully baked lighting reminiscent of *The Witness* (2016).

> **Status: planning complete, implementation not started.** See [ROADMAP.md](ROADMAP.md).

## How it will work

- **Engine:** a fork of [jer-nc/lightbaker](https://github.com/jer-nc/lightbaker) (Rust, wgpu, WGSL
  compute ray tracing over a BVH, multi-bounce gather, tile cache), compiled to
  `wasm32-unknown-unknown` and run on **WebGPU** in a Web Worker.
- **UVs:** [xatlas](https://github.com/jpcy/xatlas) generates lightmap UVs (upstream's planar
  layout stays available).
- **Lighting:** sun, sky, point, spot, area lights and emissive materials, all participating in
  light transport (`Lo = Le + albedo/π · E`).
- **Editor:** React + Three.js + Vite: hierarchy, gizmos, materials, lights, bake controls,
  baked/unbaked comparison, UV chart view, assets from [polyfork.dev](https://polyfork.dev/) or any
  GLB.

## Documentation

| Document | Contents |
|---|---|
| [AGENTS.md](AGENTS.md) | Rules and workflow for contributors and AI agents — **read first** |
| [ROADMAP.md](ROADMAP.md) | Phases, tasks, exit criteria, milestones, open questions |
| [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) | Original project brief |
| [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) | Numbered requirements and success criteria |
| [docs/FEASIBILITY.md](docs/FEASIBILITY.md) | Technical feasibility report on upstream lightbaker |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Target architecture |
| [docs/LIGHTING_MODEL.md](docs/LIGHTING_MODEL.md) | Radiometry, lights, GI algorithm, Three.js rendering rules, presets |
| [docs/INTERFACES.md](docs/INTERFACES.md) | TypeScript/Rust interfaces, worker protocol, scene format v2, export |
| [docs/TESTING.md](docs/TESTING.md) | Test strategy and deterministic scenes |
| [docs/DEMO_SCENES.md](docs/DEMO_SCENES.md) | Lighting tests 1–5 and the showcase |
| [docs/ASSET_LIBRARY.md](docs/ASSET_LIBRARY.md) | polyfork.dev findings, licensing, import pipeline |
| [docs/RESEARCH.md](docs/RESEARCH.md) | The Witness articles, xatlas, wgpu/WebGPU, sources |
| [docs/BUILD.md](docs/BUILD.md) | Toolchain and build commands |
| [docs/adr/](docs/adr/) | Architecture decision records |

## Workflow

Changes are planned and tracked with [OpenSpec](openspec/) (`/opsx-propose`, `/opsx-apply`,
`/opsx-archive`); each roadmap task is one change. See [AGENTS.md](AGENTS.md#workflow).

## Licence

MIT, as upstream lightbaker: `Copyright (c) 2026 Orbitoon` plus `Copyright (c) 2026 The
wgpu-baking contributors`. Third-party licences (xatlas and `xatlas-wasm`: MIT) will be listed in
`THIRD_PARTY_NOTICES.md`. polyfork.dev assets are loaded from their CDN at runtime (CC BY 4.0,
credited in the app) and are never part of this repository.
