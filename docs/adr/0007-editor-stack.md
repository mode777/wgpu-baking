# ADR-0007: Editor stack — imperative Three.js viewport, React UI, single document store

- **Status:** proposed (confirmed at the start of Phase 5)
- **Date:** 2026-11-20
- **Phase:** 5
- **Requirements:** REQ-APP-*, REQ-ENG-02

## Context

The brief mandates TypeScript, Three.js, React and Vite, WebGPU for baking and optionally WebGL
for viewing. An editor needs selection, gizmos, undo/redo, panels and a scene hierarchy.

## Decision

- **Viewport:** a framework-free `Viewport` class (`app/src/viewport/`) owning the Three.js scene,
  renderer, `OrbitControls`, `TransformControls`, selection (raycast) and lightmap application.
  It subscribes to the document store and reconciles Three.js objects from it.
- **Renderer:** Three.js `WebGPURenderer` (which can fall back to a WebGL2 backend) behind a small
  adapter so the classic `WebGLRenderer` can be used if a required feature (e.g. lightmap channel
  handling) is missing in the WebGPU path. The choice is pinned by the Phase 5 calibration test
  ([LIGHTING_MODEL.md §8](../LIGHTING_MODEL.md#8-runtime-rendering-in-threejs)).
- **UI:** React function components for panels (hierarchy, inspector, lights, materials, bake,
  asset browser). No react-three-fiber: the viewport is imperative and editor-specific; R3F would
  add a second reconciliation layer.
- **State:** one `SceneDocument` store (Zustand) with command-based mutations (undo/redo stack).
  The baker receives only exported `BakeScene` data, never store objects.

## Alternatives considered

| Option | Why not |
|---|---|
| react-three-fiber + drei | Productive for viewers; for an editor with gizmos, undo and bake reconciliation it mixes declarative and imperative state. Can be revisited. |
| Redux Toolkit | Heavier; Zustand + explicit commands suffice. |

## Consequences

Clear boundary: React never touches Three.js objects directly; the viewport never owns
application state.

## Verification

Phase 5 e2e flows (select, transform, undo, bake, toggle) pass in Chromium with WebGPU.
