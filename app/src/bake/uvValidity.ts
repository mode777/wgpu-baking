/**
 * Lightmap UV validity (docs/TESTING.md "UV generation tests"): what any unwrap (xatlas native or web, or the
 * planar layout) must satisfy before the bake may use it. Pure functions over typed arrays.
 */

export interface UvCheckInput {
  /** input positions (xyz) the `xref` refers to */
  positions: Float32Array;
  /** input triangles (three per triangle); absent: three vertices a triangle */
  inputIndices?: Uint32Array;
  /** input triangles' material ids (per triangle), if any */
  materialIds?: Uint32Array;
  /** output: uv per vertex (tile texels), xref per vertex, indices, chart per vertex */
  uv: Float32Array;
  xref: Uint32Array;
  indices: Uint32Array;
  chart: Int32Array;
  width: number;
  height: number;
}

export interface UvReport {
  triangles: number;
  /** UVs outside 0 … width/height */
  outOfRange: number;
  /** triangles with 3D area but (near) zero UV area */
  degenerate: number;
  /** triangles whose UV winding disagrees with the majority of their chart */
  flipped: number;
  /** output vertices whose xref position differs from the input */
  badXref: number;
  /** charts holding faces of more than one material */
  mixedCharts: number;
  /** charts' texel density (√(uv area / 3D area)) relative to the requested one: [min, median, max], tiny charts excluded */
  density: [number, number, number];
  valid: boolean;
}

export function checkUvs(c: UvCheckInput, texelsPerMeter: number): UvReport {
  const tris = c.indices.length / 3;
  let outOfRange = 0, degenerate = 0, flipped = 0, badXref = 0;
  const eps = 1e-3;
  for (let v = 0; v < c.uv.length / 2; v++) {
    const u = c.uv[v * 2]!, w = c.uv[v * 2 + 1]!;
    if (!(u >= -eps && u <= c.width + eps && w >= -eps && w <= c.height + eps)) outOfRange++;
  }
  // xref: an output vertex's position must be its input vertex's (we only get xref, so check consistency of
  // triangles: input triangle t's vertices are xref of output triangle t's vertices, in order)
  const pos = (i: number) => [c.positions[i * 3]!, c.positions[i * 3 + 1]!, c.positions[i * 3 + 2]!] as const;
  const inputVerts = c.positions.length / 3;
  const chartSign = new Map<number, [number, number]>();
  const chartArea = new Map<number, [number, number]>();
  const chartMaterial = new Map<number, Set<number>>();
  const sign: number[] = [];
  for (let t = 0; t < tris; t++) {
    const [a, b, d] = [c.indices[t * 3]!, c.indices[t * 3 + 1]!, c.indices[t * 3 + 2]!];
    // output triangle t is input triangle t: each corner's xref must sit where the input corner does
    for (let k = 0; k < 3; k++) {
      const v = c.indices[t * 3 + k]!, x = c.xref[v]!, iv = c.inputIndices ? c.inputIndices[t * 3 + k]! : t * 3 + k;
      if (x >= inputVerts) badXref++;
      else {
        const [q, r] = [pos(x), pos(iv)];
        if (Math.abs(q[0] - r[0]) + Math.abs(q[1] - r[1]) + Math.abs(q[2] - r[2]) > 1e-6) badXref++;
      }
    }
    const [p0, p1, p2] = [pos(c.xref[a]!), pos(c.xref[b]!), pos(c.xref[d]!)];
    const e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]], e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    const cross = [e1[1]! * e2[2]! - e1[2]! * e2[1]!, e1[2]! * e2[0]! - e1[0]! * e2[2]!, e1[0]! * e2[1]! - e1[1]! * e2[0]!];
    const area3 = Math.hypot(cross[0]!, cross[1]!, cross[2]!) / 2;
    const ua = (c.uv[b * 2]! - c.uv[a * 2]!) * (c.uv[d * 2 + 1]! - c.uv[a * 2 + 1]!) - (c.uv[d * 2]! - c.uv[a * 2]!) * (c.uv[b * 2 + 1]! - c.uv[a * 2 + 1]!);
    const chart = c.chart[a]!;
    sign[t] = Math.sign(ua);
    if (area3 > 1e-10 && Math.abs(ua) / 2 < 1e-6) degenerate++;
    const s = chartSign.get(chart) ?? [0, 0];
    if (ua > 0) s[0]++;
    else if (ua < 0) s[1]++;
    chartSign.set(chart, s);
    const ar = chartArea.get(chart) ?? [0, 0];
    ar[0] += Math.abs(ua) / 2;
    ar[1] += area3;
    chartArea.set(chart, ar);
    if (c.materialIds) {
      const set = chartMaterial.get(chart) ?? new Set<number>();
      set.add(c.materialIds[t]!);
      chartMaterial.set(chart, set);
    }
  }
  for (let t = 0; t < tris; t++) {
    const s = chartSign.get(c.chart[c.indices[t * 3]!]!)!;
    const majority = s[0] >= s[1] ? 1 : -1;
    if (sign[t] !== 0 && sign[t] !== majority) flipped++;
  }
  const mixedCharts = [...chartMaterial.values()].filter((s) => s.size > 1).length;
  // tiny charts (under 4 × 4 texels at the requested density) are excluded: xatlas gives every chart at least a texel
  const ratios = [...chartArea.values()].filter(([, a3]) => a3 * texelsPerMeter * texelsPerMeter >= 16).map(([uvA, a3]) => Math.sqrt(uvA / a3) / texelsPerMeter).sort((x, y) => x - y);
  const density: [number, number, number] = ratios.length ? [ratios[0]!, ratios[Math.floor(ratios.length / 2)]!, ratios[ratios.length - 1]!] : [0, 0, 0];
  return { triangles: tris, outOfRange, degenerate, flipped, badXref, mixedCharts, density, valid: outOfRange + degenerate + flipped + badXref + mixedCharts === 0 };
}
