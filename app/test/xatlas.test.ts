import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { readScene } from '@orbitoon/lightbaker';
import * as THREE from 'three';
import { beforeAll, describe, expect, it } from 'vitest';
import type { XAtlasModule } from 'xatlas-wasm';
import { initSync, unwrapScene as bridgeUnwrapScene } from 'lightbaker-web';
import { checkUvs } from '../src/bake/uvValidity';
import { loadXatlas, unwrapTile, type UnwrapMesh } from '../src/bake/xatlasUnwrapper';

let xatlas: XAtlasModule;
beforeAll(async () => {
  // the bridge (built by `pnpm build:wasm`), for the paths through the engine's unwrap
  initSync({ module: readFileSync(fileURLToPath(new URL('../../crates/lightbaker-web/pkg/lightbaker_web_bg.wasm', import.meta.url))) });
  const t = performance.now();
  xatlas = await loadXatlas();
  console.log(`xatlas-wasm loaded in ${(performance.now() - t).toFixed(0)} ms`);
});

function fromGeometry(g: THREE.BufferGeometry, materialOf?: (triangle: number) => number): UnwrapMesh {
  const positions = new Float32Array(g.getAttribute('position').array);
  const indices = new Uint32Array(g.getIndex()!.array);
  const materialIds = materialOf ? Uint32Array.from({ length: indices.length / 3 }, (_, t) => materialOf(t)) : undefined;
  return { positions, indices, normals: new Float32Array(g.getAttribute('normal').array), materialIds };
}

function unwrapAndCheck(name: string, mesh: UnwrapMesh, texelsPerMeter: number, padding = 2) {
  const before = process.memoryUsage().arrayBuffers;
  const t = performance.now();
  const tile = unwrapTile(xatlas, [mesh], { texelsPerMeter, padding });
  const ms = performance.now() - t;
  const out = tile.meshes[0]!;
  const report = checkUvs({ positions: mesh.positions, inputIndices: mesh.indices, materialIds: mesh.materialIds, ...out, width: tile.width, height: tile.height }, texelsPerMeter);
  const tris = (mesh.indices?.length ?? mesh.positions.length / 3) / 3;
  console.log(
    `${name}: ${tris} triangles → ${out.uv.length / 2} vertices, ${tile.charts} charts, tile ${tile.width}×${tile.height}, ` +
      `utilisation ${(tile.utilisation * 100).toFixed(0)} %, ${ms.toFixed(1)} ms (charts ${tile.ms.charts.toFixed(1)}, pack ${tile.ms.pack.toFixed(1)}, output ${tile.ms.output.toFixed(1)}), ` +
      `density ${report.density.map((d) => d.toFixed(2)).join('/')}, Δ arrayBuffers ${((process.memoryUsage().arrayBuffers - before) / 1048576).toFixed(1)} MiB`,
  );
  return { tile, report };
}

describe('xatlas-wasm per object tile', () => {
  it('sizes a unit quad from texels per metre and padding', () => {
    const quad: UnwrapMesh = { positions: new Float32Array([0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1]), indices: new Uint32Array([0, 2, 1, 0, 3, 2]) };
    const { tile, report } = unwrapAndCheck('quad', quad, 32);
    expect(report.valid, JSON.stringify(report)).toBe(true);
    expect(Math.max(tile.width, tile.height)).toBeGreaterThanOrEqual(32);
    expect(Math.max(tile.width, tile.height)).toBeLessThanOrEqual(32 + 2 * 2 + 4);
  });

  it('never lets a chart span materials', () => {
    const box = new THREE.BoxGeometry(2, 1, 3);
    // BoxGeometry: 2 triangles per face, 6 faces; three materials, two faces each
    const { tile, report } = unwrapAndCheck('box, 3 materials', fromGeometry(box, (t) => Math.floor(t / 4)), 16);
    expect(report.mixedCharts).toBe(0);
    expect(report.valid, JSON.stringify(report)).toBe(true);
    expect(tile.charts).toBeGreaterThanOrEqual(3);
  });

  it('gives valid UVs for the upstream-basic meshes, a sphere and a torus knot', () => {
    const s = readScene(new Uint8Array(readFileSync(fileURLToPath(new URL('../../test-scenes/upstream-basic/upstream-basic.lbscene', import.meta.url)))));
    const meshes: [string, UnwrapMesh, number][] = s.positions.map((p, i) => [`upstream-basic/${s.header.meshes[i]!.name}`, { positions: p }, s.header.lightmap.texelsPerMeter]);
    meshes.push(['sphere 64×32', fromGeometry(new THREE.SphereGeometry(1, 64, 32)), 64]);
    meshes.push(['torus knot 256×32', fromGeometry(new THREE.TorusKnotGeometry(1, 0.3, 256, 32)), 64]);
    for (const [name, mesh, tpm] of meshes) {
      const { report } = unwrapAndCheck(name, mesh, tpm);
      expect(report, name).toMatchObject({ outOfRange: 0, degenerate: 0, flipped: 0, badXref: 0, mixedCharts: 0 });
      // density within ±20 % of the request on the median chart (TESTING.md); meshes of tiny charts only have none
      if (report.density[1] > 0) {
        expect(report.density[1], name).toBeGreaterThan(0.8);
        expect(report.density[1], name).toBeLessThan(1.2);
      }
    }
  });

  it('is deterministic with itself', () => {
    const mesh = fromGeometry(new THREE.TorusKnotGeometry(1, 0.3, 64, 8));
    const a = unwrapTile(xatlas, [mesh], { texelsPerMeter: 32, padding: 2 });
    const b = unwrapTile(xatlas, [mesh], { texelsPerMeter: 32, padding: 2 });
    expect(b.width).toBe(a.width);
    expect(b.meshes[0]!.uv).toEqual(a.meshes[0]!.uv);
  });
});

describe('unwrap request in the worker core', () => {
  it('unwraps upstream-basic per mesh and replies unwrapped', async () => {
    const { createBakerCore } = await import('../src/bake/bakerCore');
    const { BakerClient } = await import('../src/bake/client');
    const { unwrapScene } = await import('../src/bake/unwrapScene');
    const worker = {
      onmessage: null as ((e: MessageEvent) => void) | null,
      postMessage: (m: unknown) => void Promise.resolve().then(() => core.handle(m as never)),
      terminate: () => {},
    };
    const bridge = { capabilities: async () => ({ webgpu: false, reason: 'node' }), create: async () => Promise.reject(new Error('no GPU in Node')) };
    const core = createBakerCore(bridge, (e) => queueMicrotask(() => worker.onmessage?.({ data: e } as MessageEvent)), undefined, (s, c) => unwrapScene(s, c, bridgeUnwrapScene));
    const client = new BakerClient(worker);
    const scene = readFileSync(fileURLToPath(new URL('../../test-scenes/upstream-basic/upstream-basic.lbscene', import.meta.url)));
    const r = await client.unwrap(scene.buffer.slice(scene.byteOffset, scene.byteOffset + scene.byteLength), { texelsPerMeter: 4, padding: 2 }).result;
    expect(r.meshes.map((m) => m.name)).toEqual(['ground', 'box', 'wall', 'strip']);
    expect(r.meshes.every((m) => m.valid)).toBe(true);
    expect(r.meshes.map((m) => m.report.badXref + m.report.flipped)).toEqual([0, 0, 0, 0]);
    expect(r.meshes.map((m) => m.triangles)).toEqual([512, 12, 12, 2]);
    expect(r.meshes[0]!.width).toBeGreaterThanOrEqual(256);
  });
});
describe('native/web equivalence (P3.6)', () => {
  it('bridge + xatlas-wasm unwrap the equivalence scene like the native fixture', async () => {
    const { unwrapScene } = await import('../src/bake/unwrapScene');
    const dir = new URL('../../test-scenes/uv-equivalence/', import.meta.url);
    const scene = new Uint8Array(readFileSync(fileURLToPath(new URL('uv-equivalence.lbscene', dir))));
    const fixture = JSON.parse(readFileSync(fileURLToPath(new URL('native.json', dir)), 'utf8')) as {
      texelsPerMeter: number;
      padding: number;
      meshes: { name: string; width: number; height: number; charts: number; triangles: number; xref: number[]; indices: number[]; uv: number[] }[];
    };
    const r = await unwrapScene(scene, { texelsPerMeter: fixture.texelsPerMeter, padding: fixture.padding }, bridgeUnwrapScene);
    expect(r.meshes.map((m) => m.name)).toEqual(fixture.meshes.map((m) => m.name));
    r.meshes.forEach((m, i) => {
      const e = fixture.meshes[i]!;
      expect(m.valid, `${m.name}: ${JSON.stringify(m.report)}`).toBe(true);
      expect({ width: m.width, height: m.height, charts: m.charts, triangles: m.triangles }, m.name).toEqual({ width: e.width, height: e.height, charts: e.charts, triangles: e.triangles });
      expect(Array.from(m.xref), `${m.name}: xref`).toEqual(e.xref);
      expect(Array.from(m.indices), `${m.name}: indices`).toEqual(e.indices);
      // the tolerance of crates/lightbaker-uv/tests/equivalence.rs (ADR-0006, P3.6)
      const worst = Array.from(m.uv).reduce((w, u, k) => Math.max(w, Math.abs(u - e.uv[k]!)), 0);
      console.log(`${m.name}: ${m.triangles} triangles, ${m.width}×${m.height}, ${m.charts} charts; UVs within ${worst} texels of native`);
      expect(worst, `${m.name}: UVs`).toBeLessThanOrEqual(0.01);
    });
  });
});