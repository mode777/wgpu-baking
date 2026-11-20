/**
 * The messages between the page and the baker worker (docs/INTERFACES.md §7; the Phase 2 subset plus the
 * PoC's additions: `init.allowFallbackAdapter`, `ready`, `bake.expect` and `done.result.checks`).
 */

export type Id = string;

export type BakePhase = 'layout' | 'direct' | 'bounce' | 'volume';

export interface AdapterInfo {
  name: string;
  vendor: number;
  device: number;
  /** `IntegratedGpu`, `DiscreteGpu`, `VirtualGpu`, `Cpu`, `Other` */
  deviceType: string;
  backend: string;
  driver: string;
  driverInfo: string;
  /** a software/fallback adapter: correct but slow; results are labelled */
  software: boolean;
  isFallbackAdapter: boolean;
  limits: { maxStorageBufferBindingSize: number; maxBufferSize: number; maxComputeWorkgroupsPerDimension: number };
}

export interface BakerCapabilities {
  webgpu: boolean;
  adapter?: AdapterInfo;
  /** why baking is unavailable, when `webgpu` is false */
  reason?: string;
}

export type BakerErrorKind = 'no-webgpu' | 'no-adapter' | 'fallback-adapter-refused' | 'device-lost' | 'out-of-memory' | 'limit-exceeded' | 'invalid-scene' | 'internal';

export interface BakerError {
  kind: BakerErrorKind;
  message: string;
}

export interface CheckOutcome {
  name: string;
  a: number;
  b: number;
  passed: boolean;
}

/** `expect.json` evaluated against a bake (docs/TESTING.md). */
export interface CheckReport {
  passed: boolean;
  values: Record<string, [number, number, number, number]>;
  outcomes: CheckOutcome[];
}

export interface BakeStats {
  triangles: number;
  charts: number;
  bake: { tiles: number; tilesBaked: number; texelsBaked: number; chunks: number; chunksBaked: number; seconds: number };
  adapter: string;
  software: boolean;
  transfers: { batches: number; readbacks: number; readbackBytes: number; uploadBytes: number };
  cacheBytes: number;
  /** the wasm heap's size after the bake */
  wasmMemoryBytes?: number;
}

/** What `WebBaker.bake` resolves to (docs/INTERFACES.md §8), plus the checks when `expect` was given. */
export interface BakeOutput {
  lbake: Uint8Array;
  atlas: { width: number; height: number };
  /** per atlas texel: sun field, AO */
  direct: Float32Array;
  /** irradiance, linear RGB, per texel of the smaller indirect atlas */
  indirect: { width: number; height: number; data: Float32Array };
  validity: Uint8Array;
  volume?: { origin: [number, number, number]; step: number; size: [number, number, number]; probes: Float32Array };
  stats: BakeStats;
  adapter: AdapterInfo;
  warnings: string[];
  checks?: CheckReport;
}

/** A partial `BakeConfig` (INTERFACES.md §4); the bridge applies what it supports and warns about the rest. */
export type BakeConfigOverrides = Record<string, unknown>;

/** `BakeConfig.lightmap` as far as the unwrap uses it. */
export interface UnwrapConfig {
  texelsPerMeter: number;
  padding: number;
  xatlas?: { maxChartArea?: number; maxBoundaryLength?: number; normalDeviationWeight?: number; roundnessWeight?: number; straightnessWeight?: number; maxIterations?: number; bruteForcePacking?: boolean };
}

export interface UnwrappedMeshResult {
  name: string;
  /** the object tile's size in texels */
  width: number;
  height: number;
  charts: number;
  utilisation: number;
  triangles: number;
  /** per output vertex: uv in tile texels (xy), input vertex, chart; three indices a triangle */
  uv: Float32Array;
  xref: Uint32Array;
  indices: Uint32Array;
  ms: number;
  /** UV validity (docs/TESTING.md) */
  valid: boolean;
}

export interface UnwrapResult {
  /** time to load xatlas in this worker (0 once loaded) */
  loadMs: number;
  meshes: UnwrappedMeshResult[];
}

/** Device loss is reported once per baker as an `error` without `jobId`; a bake running at that moment also
 * ends with its own `device-lost` error. */
/** Device loss is reported once per baker as an `error` without `jobId`; a bake running at that moment also
 * ends with its own `device-lost` error. */
export type BakeRequest =
  | { type: 'init'; allowFallbackAdapter: boolean }
  | { type: 'bake'; jobId: Id; scene: ArrayBuffer; config?: BakeConfigOverrides; expect?: string }
  /** xatlas lightmap UVs for each mesh of an `.lbscene`, each mesh one object tile (P3.1) */
  | { type: 'unwrap'; jobId: Id; scene: ArrayBuffer; config: UnwrapConfig }
  | { type: 'cancel'; jobId: Id }
  | { type: 'dispose' }
  /** debug sessions only (`?debug=1`): destroy the worker's GPU device, as a driver reset would */
  | { type: 'debug-lose-device' };

export type BakeEvent =
  | { type: 'capabilities'; caps: BakerCapabilities }
  /** the baker opened its device (after `capabilities`, when WebGPU is usable) */
  | { type: 'ready'; adapter: AdapterInfo }
  | { type: 'progress'; jobId: Id; phase: BakePhase; bounce: number; done: number; total: number; elapsedMs: number }
  | { type: 'done'; jobId: Id; result: BakeOutput; elapsedMs: number }
  | { type: 'unwrapped'; jobId: Id; result: UnwrapResult; elapsedMs: number }
  | { type: 'cancelled'; jobId: Id }
  | { type: 'error'; jobId?: Id; error: BakerError };

const KINDS: readonly string[] = ['no-webgpu', 'no-adapter', 'fallback-adapter-refused', 'device-lost', 'out-of-memory', 'limit-exceeded', 'invalid-scene', 'internal'];

/** Whatever a bridge call threw, as a `BakerError` (or `cancelled`). */
export function toBakerError(e: unknown): BakerError | { kind: 'cancelled'; message: string } {
  if (e && typeof e === 'object' && 'kind' in e && typeof (e as { kind: unknown }).kind === 'string') {
    const { kind, message } = e as { kind: string; message?: unknown };
    const text = typeof message === 'string' ? message : String(message ?? kind);
    if (kind === 'cancelled') return { kind: 'cancelled', message: text };
    if (KINDS.includes(kind)) return { kind: kind as BakerErrorKind, message: text };
  }
  return { kind: 'internal', message: e instanceof Error ? e.message : String(e) };
}
