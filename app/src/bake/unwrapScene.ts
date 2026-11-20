/**
 * Lightmap UVs for every mesh of an `.lbscene`, each mesh one object tile (ADR-0004), with xatlas (ADR-0006):
 * the bridge (`lightbaker-web` `unwrapScene`) runs the engine's unwrap and UV checks and calls `xatlas-wasm`
 * through the adapter. Runs in the baker worker; also in Node (tests).
 */

import type { JsUnwrapper } from 'lightbaker-web';
import type { UnwrapConfig, UnwrappedMeshResult, UnwrapResult } from './protocol';
import { loadXatlas, unwrapTile } from './xatlasUnwrapper';

/** The bridge's `unwrapScene` (after its module is initialised). */
export type BridgeUnwrap = (scene: Uint8Array, config: UnwrapConfig, unwrapper: JsUnwrapper) => UnwrappedMeshResult[];

let loaded = false;

export async function unwrapScene(sceneBytes: Uint8Array, config: UnwrapConfig, bridge: BridgeUnwrap, wasmUrl?: string): Promise<UnwrapResult> {
  const t0 = performance.now();
  const xatlas = await loadXatlas(wasmUrl);
  const loadMs = loaded ? 0 : performance.now() - t0;
  loaded = true;
  const meshes = bridge(sceneBytes, config, { unwrapTile: (meshes, options) => unwrapTile(xatlas, meshes, options) });
  return { loadMs, meshes };
}
