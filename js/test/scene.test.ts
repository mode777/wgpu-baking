import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readScene, writeScene } from '../src/scene';

const upstreamBasic = fileURLToPath(new URL('../../test-scenes/upstream-basic/upstream-basic.lbscene', import.meta.url));

describe('readScene', () => {
  it('reads the scene the Rust engine wrote', () => {
    const s = readScene(new Uint8Array(readFileSync(upstreamBasic)));
    expect(s.header.meshes.map((m) => m.name)).toEqual(['ground', 'box', 'wall', 'strip']);
    expect(s.header.meshes.reduce((n, m) => n + m.triangles, 0)).toBe(538);
    expect(s.positions.map((p) => p.length)).toEqual(s.header.meshes.map((m) => m.triangles * 9));
    // the ground's first vertex is its corner at the origin
    expect(Array.from(s.positions[0]!.subarray(0, 3))).toEqual([0, 0, 0]);
    expect(s.header.lightmap.texelsPerMeter).toBe(4);
  });

  it('round-trips through writeScene', () => {
    const a = readScene(new Uint8Array(readFileSync(upstreamBasic)));
    const b = readScene(writeScene(a.header, a.positions));
    expect(b.header).toEqual(a.header);
    expect(b.positions).toEqual(a.positions);
  });

  it('rejects what is not a scene', () => {
    expect(() => readScene(new Uint8Array(16))).toThrow(/not a lightbaker scene/);
    const s = readScene(new Uint8Array(readFileSync(upstreamBasic)));
    const bytes = writeScene(s.header, s.positions);
    expect(() => readScene(bytes.subarray(0, bytes.length - 8))).toThrow(/cut short/);
  });
});

// .lbscene v2 across languages: the TS writer is pinned by test-scenes/format-v2/ts.lbscene, and the file the
// Rust writer produced (rust.lbscene, crates/lightbaker/tests/format_v2.rs) reads back as the same scene.
// Regenerate with UPDATE_GOLDEN=1 pnpm -C js test.
describe('scene format v2 golden files', async () => {
  const { writeSceneV2, SCENE_VERSION_2 } = await import('../src/scene');
  const { existsSync, writeFileSync } = await import('node:fs');
  const dir = (f: string) => fileURLToPath(new URL(`../../test-scenes/format-v2/${f}`, import.meta.url));
  const header = {
    sun: { direction: [-1, 2, 0.5], radiusDeg: 0.5, irradiance: [3, 3, 3] },
    sky: { zenith: [0.25, 0.25, 0.5], horizon: [0.5, 0.5, 0.5], ground: [0.125, 0.125, 0.125] },
    materials: [
      { name: 'white', albedo: [0.75, 0.75, 0.75], receives: true, casts: true, opacity: 1 },
      { name: 'lamp', albedo: [0.5, 0.25, 0.125], receives: true, casts: true, opacity: 1, emission: [4, 2, 1], doubleSided: true },
    ],
    meshes: [
      { name: 'quad', material: 0, triangles: 0 },
      { name: 'triangle', material: 0, triangles: 0 },
    ],
    lightmap: { texelsPerMeter: 8, tileSize: 16, chartAngleDeg: 40, padding: 2, atlasWidth: 256, indirectDownscale: 2 },
    bake: { sunSamples: 4, aoRays: 16, aoReach: 0.5, aoStrength: 1, aoFloor: 0.25, aoContrast: 1, sdfRadius: 2, giRays: 32, giBounces: 2, giReach: 8, insideShare: 0.25, giFilter: 1 },
    volume: null,
    user: { source: 'golden' },
    lights: [{ type: 'point', position: [0.5, 2, 0.5], color: [1, 0.75, 0.5], intensity: 10, range: 8 }],
  } as const satisfies import('../src/scene').SceneHeader;
  const meshes = [
    {
      positions: new Float32Array([0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1]),
      normals: new Float32Array([0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0]),
      indices: new Uint32Array([0, 2, 1, 0, 3, 2]),
      materialIds: new Uint16Array([0, 1]),
      colors: new Float32Array([1, 0.5, 0.25, 0.5, 1, 0.25, 0.25, 0.5, 1, 1, 1, 1]),
      lightmapUv: new Float32Array([0, 0, 8, 0, 8, 8, 0, 8]),
    },
    { positions: new Float32Array([0, 1, 0, 0, 1, 1, 1, 1, 0]) },
  ];

  it('pins the TypeScript writer', () => {
    const bytes = writeSceneV2(structuredClone(header) as never, meshes);
    if (process.env.UPDATE_GOLDEN === '1' || !existsSync(dir('ts.lbscene'))) writeFileSync(dir('ts.lbscene'), bytes);
    expect(new Uint8Array(readFileSync(dir('ts.lbscene')))).toEqual(bytes);
  });

  it('reads the Rust writer\'s file', () => {
    const s = readScene(new Uint8Array(readFileSync(dir('rust.lbscene'))));
    expect(s.version).toBe(SCENE_VERSION_2);
    const { meshes: infos, ...rest } = s.header;
    const { meshes: _, ...want } = header;
    expect(rest).toEqual(want);
    expect(infos.map((m) => [m.name, m.triangles, m.vertices, m.arrays?.length])).toEqual([['quad', 2, 4, 6], ['triangle', 1, 3, 1]]);
    expect(s.meshes).toEqual(meshes);
    // the expanded view: the quad's two triangles through its indices
    expect(Array.from(s.positions[0]!)).toEqual([0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1]);
  });

  it('rejects bad v2 meshes', () => {
    const bad = (m: Record<string, unknown>) => () => writeSceneV2(structuredClone(header) as never, [m as never, meshes[1]!]);
    expect(bad({ ...meshes[0], colors: new Float32Array(3) })).toThrow(/colors:f32x3 has 3 values/);
    const bytes = writeSceneV2(structuredClone(header) as never, [{ ...meshes[0]!, indices: new Uint32Array([0, 2, 1, 0, 3, 4]) }, meshes[1]!]);
    expect(() => readScene(bytes)).toThrow(/index 4 out of range/);
  });
});