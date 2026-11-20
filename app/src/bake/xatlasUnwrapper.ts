/**
 * xatlas in the baker worker (ADR-0006): a thin adapter over `xatlas-wasm` with the shape of the Rust
 * `Unwrapper` contract — one xatlas atlas per object tile, typed arrays in and out, UVs in tile texels.
 */

import type { ChartOptions, XAtlasModule } from 'xatlas-wasm';

export interface UnwrapMesh {
  /** xyz per vertex (world space, metres) */
  positions: Float32Array;
  /** three per triangle; absent: three vertices a triangle */
  indices?: Uint32Array;
  normals?: Float32Array;
  /** per triangle: charts never span materials */
  materialIds?: Uint32Array;
}

export interface UnwrapOptions {
  texelsPerMeter: number;
  /** empty texels round each chart */
  padding: number;
  chart?: ChartOptions;
  /** slower, tighter packing */
  bruteForce?: boolean;
}

export interface UnwrappedMesh {
  /** xy per output vertex, in tile texels (0 … width, 0 … height) */
  uv: Float32Array;
  /** per output vertex: the input vertex it came from */
  xref: Uint32Array;
  /** three per triangle, into the output vertices (same triangle order as the input) */
  indices: Uint32Array;
  /** per output vertex: its chart within the tile */
  chart: Int32Array;
  charts: number;
}

export interface UnwrappedTile {
  width: number;
  height: number;
  meshes: UnwrappedMesh[];
  charts: number;
  /** share of the tile covered by charts */
  utilisation: number;
  ms: { charts: number; pack: number; output: number };
}

let module: Promise<XAtlasModule> | null = null;

/** Loads xatlas once (lazily). `wasmUrl`: where the bundler serves `xatlas.wasm` (default: beside the package). */
export function loadXatlas(wasmUrl?: string): Promise<XAtlasModule> {
  module ??= import('xatlas-wasm').then((m) => m.default(wasmUrl ? { locateFile: () => wasmUrl } : undefined));
  return module;
}

const now = () => performance.now();

/** Unwraps one object tile: all its meshes share the tile's rectangle. */
export function unwrapTile(xatlas: XAtlasModule, meshes: UnwrapMesh[], options: UnwrapOptions): UnwrappedTile {
  const atlas = xatlas.createAtlas();
  try {
    meshes.forEach((m, i) => {
      const err = atlas.addMesh({
        positions: m.positions,
        indices: m.indices,
        normals: m.normals,
        faceMaterialData: m.materialIds,
        meshCountHint: meshes.length,
      });
      if (err !== 0) throw new Error(`xatlas: mesh ${i}: ${xatlas.addMeshErrorString(err)}`);
    });
    const t0 = now();
    atlas.computeCharts(options.chart ?? {});
    const t1 = now();
    atlas.packCharts({ padding: options.padding, texelsPerUnit: options.texelsPerMeter, resolution: 0, bilinear: true, bruteForce: options.bruteForce ?? false });
    const t2 = now();
    if (atlas.atlasCount !== 1) throw new Error(`xatlas: expected one atlas for the tile, got ${atlas.atlasCount}`);
    const out = meshes.map((_, i) => {
      const m = atlas.getMesh(i);
      const n = m.vertexCount;
      const uv = new Float32Array(n * 2), xref = new Uint32Array(n), chart = new Int32Array(n);
      for (let v = 0; v < n; v++) {
        const o = m.vertices[v]!;
        uv[v * 2] = o.uv[0];
        uv[v * 2 + 1] = o.uv[1];
        xref[v] = o.xref;
        chart[v] = o.chartIndex;
      }
      return { uv, xref, indices: m.indices.slice(), chart, charts: m.chartCount };
    });
    const t3 = now();
    return { width: atlas.width, height: atlas.height, meshes: out, charts: atlas.chartCount, utilisation: atlas.getUtilization(0), ms: { charts: t1 - t0, pack: t2 - t1, output: t3 - t2 } };
  } finally {
    atlas.destroy();
  }
}
