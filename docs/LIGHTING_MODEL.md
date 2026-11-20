# Lighting model

This document defines what the baker computes, in which units, and how Three.js must display it.
It is normative: code that disagrees with it is a bug (or the document needs an ADR-backed update).

Requirements covered: REQ-LGT-*, REQ-GI-*, REQ-OUT-* in [REQUIREMENTS.md](REQUIREMENTS.md).

## 1. Conventions

- **Linear RGB** everywhere inside the baker. Albedo and emission colours picked in the UI or read
  from glTF (`baseColorFactor`, `emissiveFactor` are already linear in glTF; textures are sRGB) are
  converted to linear before they reach the baker.
- **Y up, metres** (upstream convention).
- **Albedo** is the material albedo multiplied by the mesh's vertex colour when present (glTF
  `COLOR_0`; polyfork assets are vertex-coloured, see [ASSET_LIBRARY.md §3](ASSET_LIBRARY.md#3-consequences-for-the-baker-important)).
  The baker stores it per triangle (average of the three vertex colours — exact for flat-shaded
  assets); per-vertex interpolation at hits is a later refinement.
- **Radiometric quantities** in arbitrary but consistent "scene units". Intensities follow the
  Three.js physically based light conventions (decay 2, no legacy lighting) so that the unbaked
  real-time preview and the baked result are directly comparable.
- The lightmap stores **irradiance `E`** (incoming, cosine-weighted, linear RGB) per texel — the
  same quantity upstream's `indirect` texture stores. A surface's outgoing diffuse radiance is
  reconstructed at runtime from its albedo.

## 2. Transport equation

For a Lambertian surface point `x` with albedo `ρ` and emitted radiance `Le`:

```
Lo(x) = Le(x) + (ρ(x) / π) · E(x)
E(x)  = E_direct(x) + ∫_Ω L_in(x, ω) · cos θ dω
```

`L_in(x, ω)` is `Lo` of the first surface hit in direction `ω` (front face), the sky radiance
`L_sky(ω)` if the ray escapes (or exceeds `giReach`), and zero for back-face hits on single-sided
geometry.

### Bounce iteration (upstream design, kept)

The baker solves the equation by Jacobi-style iteration over the whole atlas, exactly as upstream
`bake.rs` does:

```
E_0      = E_direct                                        (all analytic lights, section 3)
E_k(x)   = E_direct(x) + (π / N) · Σ_i L_{k-1}(h(x, ω_i))    ω_i cosine-distributed, k = 1 … giBounces
L_{k-1}(y) = Le(y) + (ρ(y) / π) · E_{k-1}(y)                 (read from the previous pass's atlas)
```

- `giBounces = 0`: direct light only (no sky, no emissive contribution — the UI warns).
- `giBounces = 1`: direct + sky + emissive surfaces + one diffuse bounce of direct light.
- Each additional pass adds one more bounce of everything.

**Energy conservation.** With `ρ < 1` the iteration is a convergent geometric series. Albedo is
clamped to `ρ ≤ 0.9` per channel at scene export (UI shows a warning above 0.85). The white-furnace
test in [TESTING.md](TESTING.md) checks `L_k = Le · (1 − ρ^(k+1)) / (1 − ρ)` in a closed box.

**Known approximations inherited from upstream** (documented, kept, revisited in Phase 6):

- Hits on surfaces with no lightmap texels (`receives = false`) use a fallback irradiance (the
  sky's average). Emission on such surfaces is still added exactly.
- Back-face hits return zero and count towards "inside a solid" detection. Materials flagged
  `doubleSided` (foliage, thin sheets) treat back-face hits as front faces with the flipped normal
  and are excluded from the inside test.

## 3. Light sources

All lights are static and baked. Every light type contributes to `E_direct` with shadow rays
through the BVH and therefore also feeds every bounce.

| Light | Parameters | Irradiance at `x` (normal `n`, unoccluded) | Three.js preview |
|---|---|---|---|
| Sun / directional | direction `l`, colour·intensity = `E⊥`, angular radius `α` | `E⊥ · max(0, n·l) · V_disc` | `DirectionalLight(color, intensity)` |
| Point | position, colour·intensity = `I` (radiant intensity), range `r`, optional source radius | `I · max(0, n·l) / d² · w(d, r) · V` | `PointLight(color, I, r, decay 2)` |
| Spot | point params + direction, inner angle `θi`, outer angle `θo` | point × `smoothstep(cos θo, cos θi, cos ψ)` | `SpotLight(angle = θo, penumbra = 1 − θi/θo)` |
| Rect area | centre, orientation, width, height, colour·intensity = `Le` (radiance), one-sided | `(A / N) Σ Le · cos θ · cos θ' / d² · V` | `RectAreaLight` (no shadows; preview only) |
| Disk area | centre, normal, radius, `Le` | as rect, uniform disk sampling | approximated (spot + emissive disk) |
| Sky | `zenith`, `horizon`, `ground` radiance (upstream gradient); later: HDR environment | via gather rays (section 2) | `HemisphereLight` approximation |
| Emissive material | `emission` colour × `emissionIntensity` = `Le` | via gather rays; optional NEE (section 3.2) | `material.emissive`, `emissiveIntensity` |

`w(d, r)` is Three.js' window: `w = saturate(1 − (d / r)^4)^2` when `r > 0`, else `1`, with
`1 / max(d², 0.01)` for the inverse square. Using the identical formula keeps the preview honest.

`V` is visibility: the fraction of shadow rays (to the sampled point on the light) that are not
blocked, honouring per-material `opacity` exactly as upstream's `stops()`.

### 3.1 Soft shadows

- Sun: upstream's stratified samples over the texel and the sun disc (`sunSamples`, `radiusDeg`).
- Point/spot: optional `sourceRadius` turns them into sphere lights (uniform sphere-cap sampling).
- Area lights: `areaSamples` stratified samples per texel; soft shadows emerge from sampling.

### 3.2 Emissive surfaces

- Emission is part of the material (`emission`, `emissionIntensity`), never a separate fake light.
- **Required (Phase 4):** gather/volume rays that hit a front face add `Le` of that material
  (`bounced()` in `trace.wgsl`). This makes emissive objects light their surroundings and produce
  coloured bleeding, at bounce 1 and in every later bounce.
- **Optional (Phase 6, if gather noise is too high for small emitters):** next-event estimation
  over emissive triangles (power-proportional triangle selection, shadow rays), added to
  `E_direct`; then gather rays must skip `Le` on emissive hits to avoid double counting
  (MIS or "NEE-only for direct emission" — decided in an ADR).
- The emitter's own lightmap stores only the light it *receives*; its emission is rendered by the
  material at runtime (no double counting).

## 4. Ambient occlusion (artistic control)

Gathered GI already contains physically correct occlusion of sky and bounce light. Upstream also
computes a separate AO term (`aoRays`, `aoReach`, `aoStrength`, `aoFloor`, `aoContrast`). It is kept
as a **documented artistic control** for contact shadows:

```
E_final = E_direct + lerp(1, AO, aoStrength) · E_indirect      (AO never darkens direct light)
```

Defaults are conservative (`aoStrength ≈ 0.3–0.5`, `aoFloor ≥ 0.25`) to avoid excessive darkening.
In the upstream-compatible output mode AO stays a separate channel (section 7).

## 5. Interpolation and irradiance caching

**Upstream (kept as baseline):** bounce light is gathered only on a grid every `indirectDownscale`
texels (plus at least once per chart per coarse cell), smoothed edge-aware within a chart and a
facing (`filter::smooth`), filled to the rest of the chart (`filter::fill`), and stored in a smaller
texture. Light never crosses chart boundaries or folds (normal dot < 0.9), which prevents leaks.

**Planned improvement (Phase 6), inspired by Castaño's irradiance caching for The Witness:**

1. Gather at the coarse grid; record per sample the irradiance, the normal and the **harmonic mean
   hit distance** `R_i` of its rays (cheap: the rays are already traced).
2. Mark refinement where neighbouring samples differ by more than a threshold, where `R_i` is small
   (contact, corners), across normal discontinuities, and next to chart edges.
3. Gather additional samples there (another GPU dispatch over the refined set) — repeat for at
   most `refineLevels` levels.
4. Interpolate the remaining texels with Ward-style weights
   `w_i = 1 / (‖x − x_i‖ / R_i + √(1 − n·n_i))`, only from samples in the same chart with
   `‖x − x_i‖ < k · R_i` (no leaks through walls).
5. Interpolation quality is configurable (`interpolation.errorThreshold`, `refineLevels`,
   `minSpacing`), and "off" (gather every texel) is available as ground truth for tests.

The Witness rendered **hemicubes** with the rasteriser at each cache record; we use ray-traced
cosine-weighted gathering through the existing BVH instead (ADR-0005). The visual goal, not the
original algorithm, is the target.

## 6. Denoising and filtering

- Baseline: upstream edge-aware Gaussian (`giFilter`), chart- and normal-aware, on CPU.
- Phase 6: GPU à-trous (edge-avoiding wavelet) filter over the indirect atlas with chart id,
  normal and position edge-stopping functions; separate passes for direct area-light noise.
  Never filters across charts; never touches emission or the sun's SDF.
- **Padding/dilation:** after filtering, every chart is dilated into its padding (upstream `fill`,
  rings ≥ `padding`) so bilinear filtering never reads empty texels.
- **Seams (Phase 6):** xatlas cuts create seams; a seam-stitching pass (least-squares or
  averaging of texel pairs along shared 3D edges, à la Bakery) reduces visible discontinuities.

## 7. Bake outputs

| Mode | Contents | Used by |
|---|---|---|
| `combined` (default, app) | One HDR irradiance atlas per page at full resolution: `E_direct(all lights) + AO-weighted E_indirect` (indirect upsampled from the coarse grid, chart-aware) | Three.js `lightMap` |
| `stationary-sun` (upstream-compatible) | Upstream `.lbake`: BC5 sun SDF + AO, RGB9E5 indirect; point/spot/area/emissive light is added into the indirect irradiance | Engines that keep the real-time sun |

Formats: in-app `RGBA16F` (`HalfFloatType`) textures; export as KTX2 `RGB9E5` (upstream encoder) or
`RGBA16F`; see [INTERFACES.md](INTERFACES.md#10-export-bundle).

## 8. Runtime rendering in Three.js

Goal: a baked static mesh renders `Lo = Le + (ρ / π) · E` — no more, no less.

1. **Material:** `MeshStandardMaterial` (metalness 0, roughness 1) or `MeshLambertMaterial`, with
   `lightMap` = the atlas, `lightMap.channel = 1` (lightmap UVs in the `uv1` attribute),
   `lightMapIntensity = 1`, original `map`/`color` untouched (texture UVs stay in `uv`).
2. **Texture:** linear (`LinearSRGBColorSpace` / no sRGB decode), `HalfFloatType`, `LinearFilter`,
   `generateMipmaps = false` by default. Mipmaps only with padding ≥ `2^levels` texels and dilated
   levels.
3. **No double lighting:** in baked view, lights that were baked are excluded from the real-time
   render of baked meshes (the demo is fully static: baked lights are hidden, `scene.environment`
   is not applied to baked materials, `envMapIntensity = 0`, no `aoMap`). Lights remain visible as
   editor gizmos.
4. **Emission:** `emissive`/`emissiveIntensity` stay on the material (it is outgoing radiance, not
   in the lightmap).
5. **Display:** tone mapping (AgX or ACES) and exposure are display-only; `outputColorSpace = SRGB`.
6. **Dynamic objects** (if any): lit by the irradiance volume (upstream L1 SH) via a light probe or
   a custom node; never by the lightmap.

Phase 5 includes a calibration test: a plane with albedo `ρ` and a baked constant `E` must render
`ρ · E / π` (before tone mapping) — automated for the Three.js WebGPU backend, checked manually for
the WebGL backend. This pins the assumptions in items 1–3 to the exact Three.js version used.

## 9. Quality presets (initial values, tuned in Phase 6)

| Parameter | Preview | Balanced | High Quality |
|---|---|---|---|
| `texelsPerMeter` | 4 | 8 | 16 |
| `atlasMaxSize` | 1024 | 2048 | 4096 |
| `padding` (texels) | 2 | 3 | 4 |
| `sunSamples` | 4 | 16 | 36 |
| `areaSamples` | 4 | 16 | 64 |
| `aoRays` | 16 | 64 | 128 |
| `giRays` | 64 | 256 | 1024 |
| `giBounces` | 1 | 3 | 5 |
| `indirectDownscale` | 4 | 2 | 1 |
| `interpolation` | grid | grid | adaptive |
| `denoise` | gaussian r1 | gaussian r2 | à-trous |

Advanced users can override every parameter; the preset is only the starting point.

A test-only `ci` profile (not shown in the UI) keeps CI bakes fast on software adapters: ≤ 512²
atlas, 4 sun samples, 16 AO rays, 32 GI rays, 1–2 bounces. Metric tolerances in `expect.json` are
set per profile.
