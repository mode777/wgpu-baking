# Interfaces

The contracts between the editor (React/Three.js), the baker worker, the WASM bridge and the Rust
engine. These are **drafts**: the shapes are binding for planning; field names may be refined in
the OpenSpec change that implements them (with this document updated in the same change).

Rule: the editor knows nothing about how GI is computed; the engine knows nothing about React or
Three.js. Everything crosses this boundary as plain data (JSON + typed arrays).

```
Editor (main thread)  ──postMessage──▶  Baker worker (TS)  ──wasm-bindgen──▶  lightbaker-web (Rust)  ──▶  lightbaker core
     SceneDocument          BakeRequest        scene v2 bytes + BakeConfig             Scene + settings
     ◀── BakeEvent (progress, partial previews, result, error) ──
```

## 1. Shared primitive types (TypeScript)

```ts
export type Vec3 = [number, number, number];
export type Quat = [number, number, number, number];
export type Rgb = [number, number, number];          // linear RGB unless named *Srgb
export type Id = string;                              // stable UUID
```

## 2. Editor document (app only, not seen by the baker)

```ts
export interface SceneDocument {
  version: 1;
  objects: SceneObject[];            // flat list, `parent` forms the hierarchy
  materials: EditorMaterial[];
  lights: EditorLight[];
  environment: SkySettings;
  bake: BakeConfig;                  // last used configuration (preset + overrides)
  assets: AssetRef[];                // provenance + licence of every imported asset
}

export interface SceneObject {
  id: Id; name: string; parent: Id | null;
  transform: { position: Vec3; rotation: Quat; scale: Vec3 };
  kind: 'mesh' | 'group' | 'primitive';
  mesh?: { geometry: Id; materials: Id[] };        // one material per geometry group
  primitive?: { type: 'box' | 'plane' | 'sphere' | 'cylinder' | 'cone' | 'torus'; params: Record<string, number> };
  static: boolean;                   // only static objects are baked
  castsShadow: boolean; receivesLightmap: boolean;
}

export interface EditorMaterial {
  id: Id; name: string;
  albedo: Rgb;                       // linear; UI edits sRGB and converts
  map?: Id;                          // original texture (uses `uv`, untouched by baking)
  emission: Rgb; emissionIntensity: number;
  roughness: number;                 // display only; the baker is diffuse
  opacity: number;                   // share of light stopped (upstream semantics)
  doubleSided: boolean;
}

export interface AssetRef { id: Id; source: 'polyfork' | 'gltf-file' | 'primitive'; url?: string; license: string; author?: string; }
```

## 3. Lights (shared by editor and baker)

```ts
interface LightBase { id: Id; name: string; color: Rgb; intensity: number; enabled: boolean }

export interface DirectionalLight extends LightBase { type: 'directional'; direction: Vec3; angularRadiusDeg: number }
export interface PointLight       extends LightBase { type: 'point'; position: Vec3; range: number; sourceRadius: number }
export interface SpotLight        extends LightBase { type: 'spot'; position: Vec3; direction: Vec3; innerAngleDeg: number; outerAngleDeg: number; range: number; sourceRadius: number }
export interface RectAreaLight    extends LightBase { type: 'rect'; position: Vec3; rotation: Quat; width: number; height: number }
export interface DiskAreaLight    extends LightBase { type: 'disk'; position: Vec3; normal: Vec3; radius: number }

export type Light = DirectionalLight | PointLight | SpotLight | RectAreaLight | DiskAreaLight;

export interface SkySettings { zenith: Rgb; horizon: Rgb; ground: Rgb; intensity: number }
```

Units and formulas: [LIGHTING_MODEL.md §3](LIGHTING_MODEL.md#3-light-sources). The first enabled
directional light flagged as sun maps onto upstream's `sun` (keeps the SDF path); others go into
`lights[]`.

## 4. Bake configuration

```ts
export type QualityPreset = 'preview' | 'balanced' | 'high' | 'custom';

export interface BakeConfig {
  preset: QualityPreset;
  lightmap: {
    unwrap: 'xatlas' | 'planar';         // planar = upstream chart layout
    texelsPerMeter: number;
    padding: number;                      // texels
    atlasMaxSize: number;                 // page size; more pages are created when needed
    indirectDownscale: 1 | 2 | 4;
    chartAngleDeg: number;                // planar only
    xatlas?: { maxChartArea?: number; maxBoundaryLength?: number; normalDeviationWeight?: number; roundnessWeight?: number; straightnessWeight?: number; maxIterations?: number; bruteForcePacking?: boolean };
  };
  sampling: { sunSamples: number; areaSamples: number; aoRays: number; giRays: number; volumeRays: number };
  gi: { bounces: number; reach: number; insideShare: number };
  ao: { strength: number; reach: number; floor: number; contrast: number };
  interpolation: { mode: 'grid' | 'adaptive' | 'off'; errorThreshold: number; refineLevels: number };
  denoise: { mode: 'none' | 'gaussian' | 'atrous'; radius: number };
  output: { mode: 'combined' | 'stationary-sun'; format: 'rgba16f' | 'rgb9e5'; volume?: { min: Vec3; max: Vec3; step: number } };
  incremental: { reuseTiles: boolean; progressivePreview: boolean };
  allowFallbackAdapter: boolean;          // default false: never bake on a software adapter silently (CI sets true)
}
```

Presets resolve to full `BakeConfig` values from [LIGHTING_MODEL.md §9](LIGHTING_MODEL.md#9-quality-presets-initial-values-tuned-in-phase-6).

## 5. Bake scene (what the editor exports for the baker)

Built by `app/src/bake/exportScene.ts`: static objects only, instances flattened to **world space**,
material groups preserved per triangle.

```ts
export interface BakeScene {
  sun: { direction: Vec3; radiusDeg: number; irradiance: Rgb } | null;
  sky: { zenith: Rgb; horizon: Rgb; ground: Rgb };
  lights: Light[];
  materials: BakeMaterial[];
  meshes: BakeMesh[];
}

export interface BakeMaterial {
  name: string; albedo: Rgb; emission: Rgb;            // emission = colour × intensity (radiance)
  receives: boolean; casts: boolean; opacity: number; doubleSided: boolean;
}

export interface BakeMesh {
  name: string;
  objectId: Id;                        // maps results back to the editor object
  positions: Float32Array;             // world space, xyz
  normals?: Float32Array;              // world space; face normals when absent
  indices?: Uint32Array;               // absent: three vertices a triangle (upstream v1 layout)
  materialIds: Uint16Array | number;   // per triangle, or one material for the whole mesh
  colors?: Float32Array;               // per-vertex linear RGB albedo multiplier (glTF COLOR_0; polyfork assets)
  lightmapUv?: Float32Array;           // pre-computed (e.g. cached xatlas result); else the baker unwraps
}
```

### Scene file format v2 (`.lbscene`)

Upstream v1 (magic `LBSC`, version, JSON header, non-indexed f32 positions) stays readable.
v2 keeps the same framing and adds, per mesh, a list of typed arrays named in the header:

```jsonc
// header additions (camelCase, as upstream)
{
  "lights": [ /* Light[] */ ],
  "materials": [{ "name": "...", "albedo": [..], "emission": [0,0,0], "receives": true, "casts": true, "opacity": 1, "doubleSided": false }],
  "meshes": [{ "name": "...", "material": 0, "triangles": 120, "vertices": 80,
               "arrays": ["positions:f32x3", "normals:f32x3", "indices:u32", "materialIds:u16", "colors:f32x3", "lightmapUv:f32x2"] }]
}
```

Arrays follow the header in mesh order, then array order, each 4-byte aligned. Rust (`scene.rs`)
and TS (`js/src/scene.ts`) implementations are tested against each other with golden files.

## 6. Results

```ts
export interface LightmapPage {
  width: number; height: number;
  format: 'rgba16f' | 'rgb9e5';
  data: Uint16Array | Uint32Array;     // transferred, not copied
}

export interface BakeResult {
  pages: LightmapPage[];
  /** per BakeMesh: lightmap UVs (0…1 in its page) per vertex, and its page */
  meshes: { objectId: Id; page: number; uv: Float32Array; indices?: Uint32Array }[];
  charts?: ChartDebug;                  // for the UV chart view
  validity?: Uint8Array[];              // per page, upstream VALIDITY_* codes
  volume?: { origin: Vec3; step: number; size: [number, number, number]; data: Float32Array };
  stats: { triangles: number; charts: number; tiles: number; tilesBaked: number; texels: number; seconds: number; gpuSeconds?: number; adapter: string };
}

export interface ChartDebug { page: Uint8Array /* chart id → colour per texel */; utilisation: number[] }
```

If `xatlas` splits vertices (seams), `meshes[i].indices` is the new index buffer and `uv` is
per new vertex; the editor rebuilds the geometry (`uv1` + remapped attributes) from the xatlas
`xref` mapping returned alongside.

## 7. Worker protocol

```ts
// main → worker
export type BakeRequest =
  | { type: 'init' }                                                    // → 'capabilities'
  | { type: 'unwrap'; jobId: Id; scene: ArrayBuffer; config: BakeConfig['lightmap'] }
  | { type: 'bake'; jobId: Id; scene: ArrayBuffer; config: BakeConfig }
  | { type: 'cancel'; jobId: Id }
  | { type: 'dispose' };

// worker → main
export type BakeEvent =
  | { type: 'capabilities'; caps: BakerCapabilities }
  | { type: 'progress'; jobId: Id; phase: BakePhase; done: number; total: number; elapsedMs: number; etaMs?: number; message?: string }
  | { type: 'preview'; jobId: Id; page: number; region: [number, number, number, number]; data: Uint16Array }   // progressive preview
  | { type: 'unwrapped'; jobId: Id; result: Pick<BakeResult, 'meshes' | 'charts'> }
  | { type: 'done'; jobId: Id; result: BakeResult }
  | { type: 'cancelled'; jobId: Id }
  | { type: 'error'; jobId?: Id; error: BakerError };

export type BakePhase = 'unwrap' | 'layout' | 'bvh' | 'direct' | 'lights' | 'bounce' | 'filter' | 'volume' | 'encode';

export interface BakerCapabilities {
  webgpu: boolean;
  adapter?: { vendor: string; architecture: string; description: string; isFallbackAdapter: boolean;
              software: boolean };     // isFallbackAdapter, or a known software adapter name (llvmpipe/lavapipe, SwiftShader, WARP)
  limits?: { maxStorageBufferBindingSize: number; maxBufferSize: number; maxComputeWorkgroupsPerDimension: number };
  reason?: string;                     // human-readable when webgpu is false
}

export type BakerError =
  | { kind: 'no-webgpu' | 'no-adapter' | 'fallback-adapter-refused'; message: string }
  | { kind: 'device-lost'; reason: string; message: string }
  | { kind: 'out-of-memory' | 'limit-exceeded'; message: string; detail?: unknown }
  | { kind: 'invalid-scene'; message: string }
  | { kind: 'internal'; message: string };
```

Large arrays are sent as transferables. One bake at a time per worker; a new `bake` while one runs
is rejected (`internal`) — the UI cancels first.

Implemented subset (P2.5, `app/src/bake/protocol.ts`): `init` carries `allowFallbackAdapter`; after
`capabilities` the worker sends `{ type: 'ready'; adapter }` when it opened its device, or an `error`
without `jobId` (`no-webgpu`, `fallback-adapter-refused`, …); `bake` carries `config` (partial
`BakeConfig`) and optional `expect` (an `expect.json` string) — `done.result` is the bridge output
(§8) plus `checks` when `expect` was given, and `done.elapsedMs`; `progress` carries `bounce` and
`elapsedMs` (no `etaMs`, `preview` or `unwrap` yet). Device loss arrives as an `error` with kind
`device-lost`. The protocol logic is `app/src/bake/bakerCore.ts` (tested with a fake bridge).
The browser's `GPUAdapterInfo` (vendor, architecture) completes the adapter report and also decides
whether it is a software adapter (Chrome need not set `isFallbackAdapter` for lavapipe).

## 8. WASM bridge (`crates/lightbaker-web`, Rust side)

Implemented in P2.4 (`wasm-pack build crates/lightbaker-web --target web` → `pkg/`); JS names in
camelCase:

```rust
#[wasm_bindgen] pub async fn capabilities() -> Result<JsValue, JsValue>;
// → { webgpu: boolean, adapter?: AdapterReport & { isFallbackAdapter }, reason?: string }
//   AdapterReport = { name, vendor, device, deviceType, backend, driver, driverInfo, software,
//                     limits: { maxStorageBufferBindingSize, maxBufferSize, maxComputeWorkgroupsPerDimension } }
//   Opens no device. Without `navigator.gpu`: { webgpu: false, reason }.

#[wasm_bindgen] pub struct WebBaker { /* Rc<{ Gpu, MemoryStore, cancel flag, busy flag, last bake }> */ }

#[wasm_bindgen] impl WebBaker {
    pub async fn create(allow_fallback_adapter: bool) -> Result<WebBaker, JsValue>;
    pub fn adapter(&self) -> JsValue;                                   // AdapterReport & { isFallbackAdapter }
    pub fn bake(&self, scene: Vec<u8>, config_json: String, on_progress: Option<Function>) -> Promise;
    pub fn cancel(&self);
    pub fn evaluate(&self, expect_json: &str) -> Result<JsValue, JsValue>;   // expect.json vs the last bake
    #[wasm_bindgen(js_name = clearCache)] pub fn clear_cache(&self);
    #[wasm_bindgen(js_name = cacheBytes)] pub fn cache_bytes(&self) -> f64;
    #[wasm_bindgen(js_name = setDeviceLostCallback)] pub fn set_device_lost_callback(&self, cb: Function);
    #[wasm_bindgen(js_name = loseDeviceForTesting)] pub fn lose_device_for_testing(&self);   // device.destroy()
}
```

- `bake` resolves to `{ lbake: Uint8Array, atlas: { width, height }, direct: Float32Array /* RG per atlas
  texel: sun field, AO */, indirect: { width, height, data: Float32Array /* RGB irradiance */ },
  validity: Uint8Array, volume?: { origin, step, size, probes: Float32Array /* 16 floats a probe */ },
  stats: { triangles, charts, bake: { tiles, tilesBaked, texelsBaked, chunks, chunksBaked, seconds },
  adapter, software, transfers: { batches, readbacks, readbackBytes, uploadBytes }, cacheBytes },
  adapter, warnings: string[] }` (Phase 2 PoC shape; §6 `BakeResult` with pages and per-mesh UVs comes
  with Phase 3). On the web the `.lbake`'s KTX2 textures are not zstd-supercompressed (ADR-0003).
- `config_json` is a partial `BakeConfig` (§4). Applied now: `lightmap.{texelsPerMeter, padding,
  indirectDownscale, chartAngleDeg, atlasMaxSize}`, `sampling.{sunSamples, aoRays, giRays, volumeRays}`,
  `gi.{bounces, reach, insideShare}`, `ao.{strength, reach, floor, contrast}`, `incremental.reuseTiles`.
  Anything else yields an entry in `warnings`; invalid values reject with `invalid-scene`.
- `on_progress` receives `{ phase: 'layout' | 'direct' | 'bounce' | 'volume', bounce, done, total }`
  after layout and every GPU batch.
- Rejections are `{ kind, message }` with `kind` from `BakerError` (§7) or `'cancelled'` (the worker turns
  it into a `cancelled` event). A second `bake` while one runs rejects with `internal`.
- After device loss the callback receives the reason; the baker rejects further bakes with
  `device-lost` and must be recreated.

```rust
// xatlas on the web (ADR-0006): implemented in app/src/bake/xatlasUnwrapper.ts over `xatlas-wasm`,
// loaded once in the worker before `WebBaker::create`; synchronous after loading.
#[wasm_bindgen(raw_module = "/src/bake/xatlasUnwrapper.ts")]   // final path/binding style fixed in P3.6
extern "C" {
    #[wasm_bindgen(catch)]
    fn xatlas_unwrap_tile(input: &JsValue, options: &JsValue) -> Result<JsValue, JsValue>;
}
```

## 9. Rust engine API changes (`crates/lightbaker`)

Implemented so far (P2.1–P2.2):

- `Gpu::open(&GpuOptions { allow_software }) -> impl Future<Output = Result<Gpu>>` is public; it uses
  `Backends::BROWSER_WEBGPU` under `web`, refuses software/fallback adapters unless allowed, requests
  WebGPU's default limits with `maxStorageBufferBindingSize`/`maxBufferSize` raised to the adapter's,
  and exposes `Gpu::report: AdapterReport` (name, vendor, device type, backend, driver, `software`,
  granted limits). Failures are `GpuError::{NoAdapter, SoftwareAdapterRefused, NoDevice,
  LimitExceeded}` inside `anyhow::Error` (`downcast_ref` to map to `BakerError` kinds).
  `Gpu::new()` (blocking, software refused) stays for native behind `feature = "native"`.
- `bake(scene, gpu, &options).await` is `async`; native callers wrap it with `pollster::block_on`.
  Readbacks are `map_async` + an awaited oneshot (the device is polled only natively).
- `Options::cache: Option<&dyn TileStore>` (replacing `Option<&Path>`); `Options::progress:
  Option<&dyn Fn(Progress)>` and `Options::cancel: Option<&dyn Fn() -> bool>` (P2.4; not `Sync`, so JS
  callbacks work); `Options: Default`. Cancelling fails the bake with `progress::Cancelled` within one
  GPU batch. GPU validation errors and device loss fail the bake (`GpuError::{Gpu, OutOfMemory,
  DeviceLost}`) instead of panicking; `Gpu::take_device_lost()` resolves with the reason;
  `Gpu::probe()` reports the adapter without a device. Instance/poll selection keys on
  `target_arch = "wasm32"` (features unify in workspace builds).
- `trait TileStore { fn get(&self, kind: &str, hash: &Hash) -> Option<Vec<u8>>; fn put(&self, kind: &str, hash: &Hash, bytes: &[u8]) -> Result<()>; }`
  with `FsStore` (native, zstd, upstream behaviour) and `MemoryStore` (any platform; IndexedDB
  persistence is driven from JS).

Planned:

- `WebBaker::unwrap(scene, lightmap_json)` (Phase 3).
- `trait Unwrapper { fn unwrap(&self, meshes: &[MeshInput], opts: &UnwrapOptions) -> Result<UnwrapOutput>; }`
  implemented by `lightbaker-uv` (xatlas) and by the upstream planar layout.

## 10. Export bundle

The baked scene is exported as a **GLB plus lightmaps**, documented in `docs/EXPORT_FORMAT.md`
(written in Phase 5):

- GLB: geometry with `TEXCOORD_0` (original) and `TEXCOORD_1` (lightmap UVs), original materials
  (`KHR_materials_emissive_strength` for emission), and a vendor extension
  `LIGHTBAKER_lightmap` on each mesh primitive: `{ page, intensity }` plus, on the root,
  `{ pages: [{ uri | bufferView, format, width, height }] }`.
- Lightmap pages: KTX2 (`RGB9E5`, upstream encoder) by default; optional `.hdr` (RGBE) for tools
  without KTX2 HDR support.
- A small Three.js loader plugin in `js/` applies the lightmaps according to
  [LIGHTING_MODEL.md §8](LIGHTING_MODEL.md#8-runtime-rendering-in-threejs).
- The upstream `.lbake` remains available for the `stationary-sun` mode.
