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
