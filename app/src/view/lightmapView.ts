/**
 * Shows a bake's lightmaps on the scene it was baked from: each mesh gets the bake's chart UVs (upstream's
 * planar charts, `js/src/uv.ts`) and one of the atlases as an unlit texture. Nothing is lit in Three.js:
 * the picture is the bake (AGENTS.md rule 4).
 */

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { lightmapUVs, readBake, readScene } from '@orbitoon/lightbaker';
import type { BakeOutput } from '../bake/protocol';

export type ViewMode = 'indirect' | 'sun' | 'ao';

export class LightmapView {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
  private controls: OrbitControls;
  private meshes: THREE.Mesh[] = [];
  private textures: Partial<Record<ViewMode, THREE.DataTexture>> = {};
  private mode: ViewMode = 'indirect';
  private floatLinear: boolean;

  constructor(private readonly host: HTMLElement) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 0.6;
    host.appendChild(this.renderer.domElement);
    this.floatLinear = this.renderer.extensions.has('OES_texture_float_linear');
    this.scene.background = new THREE.Color(0x202428);
    this.camera.position.set(-20, 45, 85);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.target.set(32, 0, 32);
    this.controls.update();
    new ResizeObserver(() => this.resize()).observe(host);
    this.resize();
    this.renderer.setAnimationLoop(() => this.renderer.render(this.scene, this.camera));
  }

  /** The scene's triangles, grey until a bake is shown. */
  setScene(sceneBytes: Uint8Array): void {
    for (const m of this.meshes) this.scene.remove(m);
    const s = readScene(sceneBytes);
    this.meshes = s.positions.map((p, i) => {
      const g = new THREE.BufferGeometry();
      g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(p), 3));
      const mesh = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: 0x777777, side: THREE.DoubleSide }));
      mesh.name = s.header.meshes[i]!.name;
      this.scene.add(mesh);
      return mesh;
    });
  }

  /** Applies a bake: chart UVs per mesh, and the three atlases as textures. */
  async showBake(sceneBytes: Uint8Array, out: BakeOutput): Promise<void> {
    const bake = await readBake(out.lbake.slice().buffer);
    if (!bake) throw new Error('the bake could not be read');
    const s = readScene(sceneBytes);
    const { width, height } = out.atlas;
    let first = 0;
    s.positions.forEach((p, i) => {
      const uv = lightmapUVs(p, first, bake.triangleCharts, bake.charts, width, height);
      first += s.header.meshes[i]!.triangles;
      this.meshes[i]?.geometry.setAttribute('uv', new THREE.BufferAttribute(uv, 2));
    });
    for (const t of Object.values(this.textures)) t.dispose();
    const n = width * height;
    const sun = new Float32Array(n * 4), ao = new Float32Array(n * 4);
    for (let i = 0; i < n; i++) {
      sun.fill(out.direct[i * 2]!, i * 4, i * 4 + 3);
      ao.fill(out.direct[i * 2 + 1]!, i * 4, i * 4 + 3);
      sun[i * 4 + 3] = ao[i * 4 + 3] = 1;
    }
    const ind = out.indirect, m = ind.width * ind.height, indirect = new Float32Array(m * 4);
    for (let i = 0; i < m; i++) {
      indirect.set(ind.data.subarray(i * 3, i * 3 + 3), i * 4);
      indirect[i * 4 + 3] = 1;
    }
    this.textures = { sun: this.texture(sun, width, height), ao: this.texture(ao, width, height), indirect: this.texture(indirect, ind.width, ind.height) };
    this.setMode(this.mode);
  }

  setMode(mode: ViewMode): void {
    this.mode = mode;
    const map = this.textures[mode] ?? null;
    for (const m of this.meshes) {
      const mat = m.material as THREE.MeshBasicMaterial;
      mat.map = map;
      mat.color.set(map ? 0xffffff : 0x777777);
      // irradiance is above 1 where the sun shines: tone-map it; fields and AO are shown as they are
      mat.toneMapped = mode === 'indirect';
      mat.needsUpdate = true;
    }
  }

  private texture(data: Float32Array, w: number, h: number): THREE.DataTexture {
    const t = new THREE.DataTexture(data, w, h, THREE.RGBAFormat, THREE.FloatType);
    t.colorSpace = THREE.LinearSRGBColorSpace;
    t.magFilter = t.minFilter = this.floatLinear ? THREE.LinearFilter : THREE.NearestFilter;
    t.needsUpdate = true;
    return t;
  }

  private resize(): void {
    const { clientWidth: w, clientHeight: h } = this.host;
    if (!w || !h) return;
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }
}
