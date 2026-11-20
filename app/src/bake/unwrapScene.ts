/**
 * Lightmap UVs for every mesh of an `.lbscene`, each mesh one object tile (ADR-0004), with xatlas (ADR-0006).
 * Runs in the baker worker; also in Node (tests).
 */

import { readScene } from '@orbitoon/lightbaker';
import type { UnwrapConfig, UnwrapResult } from './protocol';
import { checkUvs } from './uvValidity';
import { loadXatlas, unwrapTile } from './xatlasUnwrapper';

let loaded = false;

export async function unwrapScene(sceneBytes: Uint8Array, config: UnwrapConfig, wasmUrl?: string): Promise<UnwrapResult> {
  const t0 = performance.now();
  const xatlas = await loadXatlas(wasmUrl);
  const loadMs = loaded ? 0 : performance.now() - t0;
  loaded = true;
  const scene = readScene(sceneBytes);
  const x = config.xatlas ?? {};
  const meshes = scene.positions.map((positions, i) => {
    const t = performance.now();
    const tile = unwrapTile(xatlas, [{ positions }], {
      texelsPerMeter: config.texelsPerMeter,
      padding: config.padding,
      chart: { maxChartArea: x.maxChartArea, maxBoundaryLength: x.maxBoundaryLength, normalDeviationWeight: x.normalDeviationWeight, roundnessWeight: x.roundnessWeight, straightnessWeight: x.straightnessWeight, maxIterations: x.maxIterations },
      bruteForce: x.bruteForcePacking,
    });
    const m = tile.meshes[0]!;
    const report = checkUvs({ positions, ...m, width: tile.width, height: tile.height }, config.texelsPerMeter);
    return { name: scene.header.meshes[i]!.name, width: tile.width, height: tile.height, charts: tile.charts, utilisation: tile.utilisation, triangles: m.indices.length / 3, uv: m.uv, xref: m.xref, indices: m.indices, ms: performance.now() - t, valid: report.valid };
  });
  return { loadMs, meshes };
}
