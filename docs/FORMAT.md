# Files

## Scene (`.lbscene`, version 1)

`LBSC`, version (u32 LE), header length (u32), header (JSON), then each mesh's positions: f32 x y z,
three vertices a triangle (not indexed), each array on a 4-byte boundary, in header order. Y up, metres.

Header (`crates/lightbaker/src/scene.rs`, `js/src/scene.ts`):

| field | |
|---|---|
| `sun` | `direction` (towards the sun), `radiusDeg` (disc), `irradiance` (linear RGB) |
| `sky` | radiance: `zenith`, `horizon` (linear in height between them), `ground` (below the horizon) |
| `materials[]` | `name`, `albedo`, `receives` (laid out and baked), `casts`, `opacity` (share of light stopped) |
| `meshes[]` | `name`, `material`, `triangles` |
| `lightmap` | `texelsPerMeter`, `tileSize`, `chartAngleDeg`, `padding`, `atlasWidth`, `indirectDownscale` (1, 2, 4) |
| `bake` | `sunSamples` (square), `aoRays`, `aoReach`, `aoStrength`, `aoFloor`, `aoContrast`, `sdfRadius`, `giRays`, `giBounces`, `giReach`, `insideShare`, `giFilter` |
| `volume` | `min`, `max`, `step`, `rays` (optional) |
| `user` | copied to the bake untouched |

## Scene (`.lbscene`, version 2)

Same framing with version 2 (ADR-0010). Header as version 1, plus:

| field | |
|---|---|
| `lights[]` | point, spot and area lights (INTERFACES.md §3); default empty. **Refused by the bake until P4.1.** |
| `materials[].emission` | emitted radiance (linear RGB), default `[0,0,0]`. **Refused if not black until P4.2.** |
| `materials[].doubleSided` | default `false`. **Refused if true until P4.3.** |
| `meshes[].vertices` | vertices in each per-vertex array (required in v2) |
| `meshes[].arrays` | the arrays that follow, `name:type`, in order (required in v2; `positions:f32x3` required) |
| `meshes[].material` | material of triangles without a material id |
| `meshes[].triangles` | must equal `indices / 3`, or `vertices / 3` without indices |

Arrays follow the header in mesh order, then in each mesh's `arrays` order, each starting on a 4-byte
boundary (zero padding):

| array | type | length | meaning |
|---|---|---|---|
| `positions:f32x3` | f32 | 3 × vertices | world space, metres |
| `normals:f32x3` | f32 | 3 × vertices | world space (used from P3.4) |
| `indices:u32` | u32 | 3 × triangles | into the vertices; absent: three vertices a triangle |
| `materialIds:u16` | u16 | triangles | per triangle, into `materials` |
| `colors:f32x3` | f32 | 3 × vertices | linear RGB albedo multiplier (used from P4.3) |
| `lightmapUv:f32x2` | f32 | 2 × vertices | given lightmap UVs (used from P3.6) |

Readers reject unknown array names, arrays of the wrong length, indices ≥ `vertices` and material ids
without a material. Defaults (`emission`, `doubleSided`, empty `lights`) are not written, so a version 1
header serialises unchanged. The engine sees a v2 mesh with several material ids as one engine mesh per
material (named `mesh#material`), each remembering its file mesh and triangles (`Scene::origin`).
Golden files: `test-scenes/format-v2/rust.lbscene` (Rust writer) and `ts.lbscene` (TypeScript writer),
each read by the other language's tests.

## Bake (`.lbake`, version 1)

`LBAK`, version, header length, header (JSON), then chunks on 8-byte boundaries (offsets from the
first chunk, which starts 8-aligned after the header).

Header: `baker`, `scene` (blake3 of the scene file), `user`, `atlas` (`width`, `height`,
`texelsPerMeter`, `sdfRadius`, `indirectWidth`, `indirectHeight`, `charts`), `skyIrradiance` (the sky
on an open face looking up), `meshes`, `volume` (`origin`, `step`, `size`, `scale`), `chunks`, `stats`.

| chunk | |
|---|---|
| `direct` | KTX2, BC5, zstd: red the sun's signed distance field (0.5 on a shadow's edge, 0 and 1 `sdfRadius` texels in and out), green ambient occlusion |
| `indirect` | KTX2, RGB9E5, zstd: irradiance from the sky and other surfaces |
| `charts` | raw deflate, f32: two rows (x y z constant) a chart, world point to atlas texel coordinates |
| `triangleCharts` | raw deflate, u32: each triangle's chart, every mesh in order (0xffffffff: none) |
| `validity` | raw deflate, u8 a texel: how it got its light (0 none, 1 its centre, 2 a point across it, 3 pushed off the surface, 4 filled from its chart, 5 the volume's), for tools |
| `volume` | raw deflate, 16 bytes a probe (x fastest, then z, then y): L0 rgb as `scale * (b/255)^2`, sun share, L1 x/y/z rgb as share of L0 times sqrt 3 (`b/255*2-1`), 3 unused |

UVs: `uv = (row0 · (p, 1) / width, row1 · (p, 1) / height)` for each vertex `p` of a triangle with its chart.
