# Demo and test scenes

Requirements covered: REQ-DEMO-01 … 06. Each lighting demonstration exists twice:

1. a **minimal deterministic test scene** in `test-scenes/` (Phase 4, numeric assertions, see
   [TESTING.md](TESTING.md)), and
2. a **showcase vignette** inside the final environment (Phase 6, art-directed).

## Art direction (the "Witness-like" target)

- Large flat surfaces with saturated but not neon solid colours (albedo 0.3–0.85 linear), minimal
  textures; geometry reads through light, not detail.
- Strong warm sun with a small angular radius (0.5–1.5°) → crisp-but-soft shadow edges; bright,
  slightly blue sky providing cool fill.
- Multiple bounces (3–5) so shadowed areas are coloured by their surroundings, never grey/black.
- Subtle AO only at contacts; no dark halos.
- Low-poly vegetation (cards with `doubleSided`, faceted canopies with partial `opacity`), rocks as
  chunky faceted solids, simple architecture (walls, arches, stairs, plinths, columns).
- Tone mapping: AgX or ACES with mild exposure; colour grading only in display.

Reference: screenshots of *The Witness* are used for comparison only and are **not** committed to
the repository (copyright).

## Test 1 — Colour bleeding

- Layout: white floor and a white back wall; three free-standing walls (red `0.8,0.05,0.05`,
  green `0.05,0.7,0.1`, blue `0.05,0.1,0.8`) at 1 m distance from neutral surfaces, lit by the sun.
- Expected: tinted indirect light on the white floor/wall next to each coloured wall, decreasing
  with distance.
- Metric: chroma of the white surface 0.5 m from each wall shifted towards that wall's hue by more
  than a set margin compared to 5 m away (upstream `tests/bake.rs` already asserts this for red).

## Test 2 — Emissive lighting

- Layout: closed dark room (albedo 0.5 grey, no openings), a green emissive cube, an orange
  emissive strip and a small white emissive panel; no analytic lights, sky blocked.
- Expected: the room is lit only through GI by the emitters; coloured pools around each emitter.
- Metric: floor irradiance near each emitter ≫ far corners; colour ratio matches emitter colour;
  with emission set to zero the room is black (proves no fake ambient).

## Test 3 — Point lights

- Layout: colonnade/arcade with an inner courtyard at dusk (weak sky), three coloured point lights
  (warm, magenta, cyan) and one spotlight.
- Expected: sharp-ish shadows from columns (small `sourceRadius`), inverse-square falloff,
  coloured indirect light on ceilings and opposite walls.
- Metric: falloff check against the analytic law at chosen points; ceiling (not directly lit)
  receives non-zero coloured irradiance only with `bounces ≥ 1`.

## Test 4 — Multiple bounces

- Layout: enclosed room with orange, teal and white walls; sunlight through a single window.
- Demonstration: the app renders the same view with `bounces = 0, 1, 2, 3, 5` (a bounce slider in
  the demo panel triggers rebakes, Preview quality).
- Metric: mean indirect irradiance increases monotonically and converges; no texel exceeds the
  energy bound.

## Test 5 — Outdoor environment

- Layout: island-like terrain (faceted low-poly), stylized trees (trunk + faceted canopy),
  rocks, a small temple/house with an arch and stairs, coloured props (spheres, pyramids, plinths).
- Lighting: sun + sky gradient; optional sunset variant.
- Expected: soft sun shadows, sky-lit blue-ish shadows, warm bounce under arches, coloured
  bleeding from props onto the ground.

## Showcase environment (Phase 6)

A single navigable scene combining Tests 1–5 as vignettes around a central plaza, sized for a
Balanced-preset bake in reasonable time on an integrated GPU (target set in Phase 6 after
measurements; tracked in the roadmap). Assets: polyfork.dev models referenced by CDN URL and
loaded at runtime (credited, never committed — see [ASSET_LIBRARY.md](ASSET_LIBRARY.md)), plus
editor primitives and our own models.

Deliverables:

- `app/public/scenes/showcase.json` (SceneDocument) and the assets it references.
- A "Load showcase" action and camera bookmarks per vignette.
- Before/after (unbaked/baked) comparisons and a "lightmap only" view for each vignette.
