/**
 * The scene a bake reads (`.lbscene`, docs/FORMAT.md): the magic `LBSC`, the format version, the
 * header's length, the header (JSON), then each mesh's positions (f32, three vertices a triangle, not
 * indexed), each on a 4-byte boundary. Y up, metres. Version 2 names, per mesh, its vertex count and typed
 * arrays (`positions:f32x3` and optionally `normals:f32x3`, `indices:u32`, `materialIds:u16`, `colors:f32x3`,
 * `lightmapUv:f32x2`), which follow in mesh order, then array order, each on a 4-byte boundary.
 */

export const SCENE_VERSION = 1;
export const SCENE_VERSION_2 = 2;

export type Rgb = [number, number, number];

export interface SceneHeader {
  sun: { direction: [number, number, number]; radiusDeg: number; irradiance: Rgb };
  /** radiance of the sky from the horizon to the zenith, and of the ground below the horizon */
  sky: { zenith: Rgb; horizon: Rgb; ground: Rgb };
  materials: SceneMaterial[];
  meshes: SceneMeshInfo[];
  lightmap: LightmapSettings;
  bake: BakeSettings;
  volume?: VolumeSettings | null;
  /** anything wanted back with the bake, copied untouched */
  user?: unknown;
  /** v2: point, spot and area lights (INTERFACES.md §3; baked from P4.1) */
  lights?: unknown[];
}

export interface SceneMeshInfo {
  name: string;
  /** its material (v2: of triangles without a material id) */
  material: number;
  triangles: number;
  /** v2: vertices in each per-vertex array */
  vertices?: number;
  /** v2: `name:type` of the arrays that follow, in order */
  arrays?: string[];
}

export interface SceneMaterial {
  name: string;
  /** diffuse reflectance, linear RGB */
  albedo: Rgb;
  receives: boolean;
  casts: boolean;
  /** share of light stopped where it casts (leaves: below one) */
  opacity?: number;
  /** v2: emitted radiance, linear RGB (baked from P4.2) */
  emission?: Rgb;
  /** v2: lit and casting from both sides (baked from P4.3) */
  doubleSided?: boolean;
}

export interface LightmapSettings {
  texelsPerMeter: number;
  tileSize: number;
  chartAngleDeg: number;
  padding: number;
  atlasWidth: number;
  indirectDownscale: number;
}

export interface BakeSettings {
  sunSamples: number;
  aoRays: number;
  aoReach: number;
  aoStrength: number;
  aoFloor: number;
  aoContrast: number;
  sdfRadius: number;
  giRays: number;
  giBounces: number;
  giReach: number;
  insideShare: number;
  giFilter: number;
}

export interface VolumeSettings {
  min: [number, number, number];
  max: [number, number, number];
  step: number;
  rays: number;
}

const align4 = (n: number) => (n + 3) & ~3;

/** The file's bytes. `positions[i]` holds mesh i's vertices (x y z, three a triangle). */
export function writeScene(header: SceneHeader, positions: readonly Float32Array[]): Uint8Array {
  if (positions.length !== header.meshes.length) throw new Error('scene: one positions array a mesh');
  header.meshes.forEach((m, i) => {
    if (positions[i]!.length !== m.triangles * 9) throw new Error(`scene: mesh ${m.name} has ${positions[i]!.length / 9} triangles, says ${m.triangles}`);
  });
  const head = new TextEncoder().encode(JSON.stringify(header));
  let size = align4(12 + head.length);
  for (const p of positions) size = align4(size + p.byteLength);
  const out = new Uint8Array(size), dv = new DataView(out.buffer);
  out.set([0x4c, 0x42, 0x53, 0x43]);
  dv.setUint32(4, SCENE_VERSION, true);
  dv.setUint32(8, head.length, true);
  out.set(head, 12);
  let at = align4(12 + head.length);
  for (const p of positions) {
    out.set(new Uint8Array(p.buffer, p.byteOffset, p.byteLength), at);
    at = align4(at + p.byteLength);
  }
  return out;
}

/** A mesh as a v2 file holds it (a v1 mesh: `positions` only, three vertices a triangle). */
export interface SceneMesh {
  /** xyz per vertex */
  positions: Float32Array;
  /** xyz per vertex */
  normals?: Float32Array;
  /** three per triangle; absent: three vertices a triangle */
  indices?: Uint32Array;
  /** per triangle */
  materialIds?: Uint16Array;
  /** linear RGB per vertex (albedo multiplier) */
  colors?: Float32Array;
  /** uv per vertex */
  lightmapUv?: Float32Array;
}

export interface Scene {
  version: number;
  header: SceneHeader;
  /** per mesh: its triangles' vertices (x y z, three a triangle; v2 meshes expanded through their indices) */
  positions: Float32Array[];
  /** per mesh, as in the file */
  meshes: SceneMesh[];
}

const ARRAYS: Record<string, { key: keyof SceneMesh; per: 'vertex' | 'corner' | 'triangle'; width: number; type: 'f32' | 'u32' | 'u16' }> = {
  'positions:f32x3': { key: 'positions', per: 'vertex', width: 3, type: 'f32' },
  'normals:f32x3': { key: 'normals', per: 'vertex', width: 3, type: 'f32' },
  'indices:u32': { key: 'indices', per: 'corner', width: 1, type: 'u32' },
  'materialIds:u16': { key: 'materialIds', per: 'triangle', width: 1, type: 'u16' },
  'colors:f32x3': { key: 'colors', per: 'vertex', width: 3, type: 'f32' },
  'lightmapUv:f32x2': { key: 'lightmapUv', per: 'vertex', width: 2, type: 'f32' },
};
const ORDER = Object.keys(ARRAYS);

function triangles(m: SceneMesh): number {
  return m.indices ? m.indices.length / 3 : m.positions.length / 9;
}

/** A v2 file. The header's meshes give names and default materials; `triangles`, `vertices` and `arrays` are set from `meshes`. */
export function writeSceneV2(header: SceneHeader, meshes: readonly SceneMesh[]): Uint8Array {
  if (meshes.length !== header.meshes.length) throw new Error('scene: one mesh a header mesh');
  const h: SceneHeader = { ...header, meshes: header.meshes.map((info, i) => {
    const m = meshes[i]!, n = m.positions.length / 3;
    const arrays = ORDER.filter((name) => m[ARRAYS[name]!.key] !== undefined);
    for (const name of arrays) {
      const a = ARRAYS[name]!, len = m[a.key]!.length;
      const want = a.per === 'vertex' ? n * a.width : a.per === 'corner' ? len - (len % 3) : triangles(m);
      if (len !== want) throw new Error(`scene: mesh ${info.name}: ${name} has ${len} values, expected ${want}`);
    }
    if (!m.indices && n % 3 !== 0) throw new Error(`scene: mesh ${info.name}: not whole triangles`);
    return { ...info, triangles: triangles(m), vertices: n, arrays };
  }) };
  const head = new TextEncoder().encode(JSON.stringify(h));
  const parts: Uint8Array[] = [];
  for (let i = 0; i < meshes.length; i++) for (const name of h.meshes[i]!.arrays!) {
    const v = meshes[i]![ARRAYS[name]!.key]!;
    parts.push(new Uint8Array(v.buffer, v.byteOffset, v.byteLength));
  }
  let size = align4(12 + head.length);
  for (const p of parts) size = align4(size + p.byteLength);
  const out = new Uint8Array(size), dv = new DataView(out.buffer);
  out.set([0x4c, 0x42, 0x53, 0x43]);
  dv.setUint32(4, SCENE_VERSION_2, true);
  dv.setUint32(8, head.length, true);
  out.set(head, 12);
  let at = align4(12 + head.length);
  for (const p of parts) {
    out.set(p, at);
    at = align4(at + p.byteLength);
  }
  return out;
}

/** Reads a scene file (v1 or v2) written by `writeScene`, `writeSceneV2` or the Rust engine. */
export function readScene(bytes: Uint8Array): Scene {
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (bytes.length < 12 || String.fromCharCode(...bytes.subarray(0, 4)) !== 'LBSC') throw new Error('scene: not a lightbaker scene');
  const version = dv.getUint32(4, true);
  if (version !== SCENE_VERSION && version !== SCENE_VERSION_2) throw new Error(`scene: format ${version}, this reader reads ${SCENE_VERSION} and ${SCENE_VERSION_2}`);
  const len = dv.getUint32(8, true);
  if (12 + len > bytes.length) throw new Error('scene: header cut short');
  const header = JSON.parse(new TextDecoder().decode(bytes.subarray(12, 12 + len))) as SceneHeader;
  let at = align4(12 + len);
  // a copy keeps every array aligned whatever the file's offset in its buffer
  const take = (n: number, type: 'f32' | 'u32' | 'u16', what: string) => {
    const size = n * (type === 'u16' ? 2 : 4);
    if (at + size > bytes.length) throw new Error(`scene: ${what}: cut short`);
    const raw = bytes.slice(at, at + size).buffer;
    at = align4(at + size);
    return type === 'f32' ? new Float32Array(raw) : type === 'u32' ? new Uint32Array(raw) : new Uint16Array(raw);
  };
  const meshes = header.meshes.map((m): SceneMesh => {
    if (m.material >= header.materials.length) throw new Error(`scene: mesh ${m.name}: no material ${m.material}`);
    if (version === SCENE_VERSION) return { positions: take(m.triangles * 9, 'f32', `mesh ${m.name}: positions`) as Float32Array };
    if (m.vertices === undefined || !m.arrays) throw new Error(`scene: mesh ${m.name}: v2 meshes need vertices and arrays`);
    const mesh: Partial<Record<keyof SceneMesh, Float32Array | Uint32Array | Uint16Array>> = {};
    for (const name of m.arrays) {
      const a = ARRAYS[name];
      if (!a) throw new Error(`scene: mesh ${m.name}: unknown array \`${name}\``);
      const n = a.per === 'vertex' ? m.vertices * a.width : a.per === 'corner' ? m.triangles * 3 : m.triangles;
      mesh[a.key] = take(n, a.type, `mesh ${m.name}: ${name}`);
    }
    const out = mesh as unknown as SceneMesh;
    if (!out.positions) throw new Error(`scene: mesh ${m.name}: no positions`);
    if (out.indices) {
      const bad = out.indices.find((i) => i >= m.vertices!);
      if (bad !== undefined) throw new Error(`scene: mesh ${m.name}: index ${bad} out of range (it has ${m.vertices} vertices)`);
    } else if (m.vertices !== m.triangles * 3) throw new Error(`scene: mesh ${m.name}: ${m.vertices} vertices without indices is not ${m.triangles} triangles`);
    const badMat = out.materialIds?.find((id) => id >= header.materials.length);
    if (badMat !== undefined) throw new Error(`scene: mesh ${m.name}: material id ${badMat} has no material`);
    return out;
  });
  if (at !== bytes.length) throw new Error(`scene: ${bytes.length - at} bytes after the last mesh`);
  const positions = meshes.map((m) => {
    if (!m.indices) return m.positions;
    const p = new Float32Array(m.indices.length * 3);
    m.indices.forEach((v, i) => p.set(m.positions.subarray(v * 3, v * 3 + 3), i * 3));
    return p;
  });
  return { version, header, positions, meshes };
}