/**
 * The baker worker's message handling, independent of the worker and of wasm: a `Bridge` (the
 * `lightbaker-web` API, or a fake in tests) and a `post` function. One bake at a time (INTERFACES.md §7).
 */

import { toBakerError, type AdapterInfo, type BakeEvent, type BakeOutput, type BakeRequest, type BakerCapabilities, type BakePhase, type CheckReport, type Id, type UnwrapConfig, type UnwrapResult } from './protocol';

export interface BridgeProgress {
  phase: BakePhase;
  bounce: number;
  done: number;
  total: number;
}

/** One opened baker (the bridge's `WebBaker`). */
export interface BridgeBaker {
  bake(scene: Uint8Array, configJson: string, onProgress: (p: BridgeProgress) => void): Promise<BakeOutput>;
  cancel(): void;
  evaluate(expectJson: string): CheckReport;
  adapter(): AdapterInfo;
  setDeviceLostCallback(cb: (reason: string) => void): void;
  /** `device.destroy()` (the bridge's `loseDeviceForTesting`) */
  loseDevice?(): void;
  free?(): void;
}

export interface Bridge {
  capabilities(): Promise<BakerCapabilities>;
  create(allowFallbackAdapter: boolean): Promise<BridgeBaker>;
}

export type Post = (event: BakeEvent, transfer?: Transferable[]) => void;

/** Lightmap UVs for a scene's meshes (`unwrapScene` in the worker; a fake in tests). */
export type SceneUnwrapper = (scene: Uint8Array, config: UnwrapConfig) => Promise<UnwrapResult>;

/** The result's typed arrays, handed over to the page instead of copied. */
export function transferables(r: BakeOutput): Transferable[] {
  const t: ArrayBufferLike[] = [r.lbake.buffer, r.direct.buffer, r.indirect.data.buffer, r.validity.buffer];
  if (r.volume) t.push(r.volume.probes.buffer);
  return [...new Set(t)].filter((b): b is ArrayBuffer => b instanceof ArrayBuffer);
}

export function createBakerCore(bridge: Bridge, post: Post, now: () => number = () => performance.now(), unwrapper?: SceneUnwrapper) {
  let baker: BridgeBaker | null = null;
  let running: Id | null = null;

  async function unwrap(jobId: Id, scene: ArrayBuffer, config: UnwrapConfig) {
    if (!unwrapper) {
      post({ type: 'error', jobId, error: { kind: 'internal', message: 'this worker has no unwrapper' } });
      return;
    }
    if (running !== null) {
      post({ type: 'error', jobId, error: { kind: 'internal', message: `job ${running} is still running: cancel it first` } });
      return;
    }
    running = jobId;
    const start = now();
    try {
      const result = await unwrapper(new Uint8Array(scene), config);
      const transfer = result.meshes.flatMap((m) => [m.uv.buffer, m.xref.buffer, m.indices.buffer]).filter((b): b is ArrayBuffer => b instanceof ArrayBuffer);
      post({ type: 'unwrapped', jobId, result, elapsedMs: now() - start }, [...new Set(transfer)]);
    } catch (e) {
      const err = toBakerError(e);
      post({ type: 'error', jobId, error: err.kind === 'cancelled' ? { kind: 'internal', message: err.message } : err });
    } finally {
      running = null;
    }
  }

  async function init(allowFallbackAdapter: boolean) {
    const caps = await bridge.capabilities().catch((e: unknown): BakerCapabilities => ({ webgpu: false, reason: toBakerError(e).message }));
    post({ type: 'capabilities', caps });
    if (!caps.webgpu) {
      post({ type: 'error', error: { kind: 'no-webgpu', message: caps.reason ?? 'WebGPU is unavailable' } });
      return;
    }
    try {
      const b = await bridge.create(allowFallbackAdapter);
      b.setDeviceLostCallback((reason) => {
        if (baker !== b) return;
        baker = null;
        post({ type: 'error', error: { kind: 'device-lost', message: `the GPU device was lost (${reason}); recreate the baker` } });
      });
      baker = b;
      post({ type: 'ready', adapter: b.adapter() });
    } catch (e) {
      const err = toBakerError(e);
      post({ type: 'error', error: err.kind === 'cancelled' ? { kind: 'internal', message: err.message } : err });
    }
  }

  async function bake(jobId: Id, scene: ArrayBuffer, config: unknown, expect: string | undefined) {
    if (!baker) {
      post({ type: 'error', jobId, error: { kind: 'internal', message: 'no baker: initialise first (or recreate it after a device loss)' } });
      return;
    }
    if (running !== null) {
      post({ type: 'error', jobId, error: { kind: 'internal', message: `bake ${running} is still running: cancel it first` } });
      return;
    }
    running = jobId;
    const start = now();
    try {
      const result = await baker.bake(new Uint8Array(scene), JSON.stringify(config ?? {}), (p) => post({ type: 'progress', jobId, phase: p.phase, bounce: p.bounce, done: p.done, total: p.total, elapsedMs: now() - start }));
      if (expect !== undefined) result.checks = baker.evaluate(expect);
      post({ type: 'done', jobId, result, elapsedMs: now() - start }, transferables(result));
    } catch (e) {
      const err = toBakerError(e);
      if (err.kind === 'cancelled') post({ type: 'cancelled', jobId });
      else post({ type: 'error', jobId, error: err });
    } finally {
      running = null;
    }
  }

  return {
    async handle(msg: BakeRequest): Promise<void> {
      switch (msg.type) {
        case 'init':
          return init(msg.allowFallbackAdapter);
        case 'bake':
          return bake(msg.jobId, msg.scene, msg.config, msg.expect);
        case 'unwrap':
          return unwrap(msg.jobId, msg.scene, msg.config);
        case 'cancel':
          if (running === msg.jobId) baker?.cancel();
          return;
        case 'dispose':
          baker?.free?.();
          baker = null;
          return;
        case 'debug-lose-device':
          baker?.loseDevice?.();
          return;
      }
    },
    get busy() {
      return running !== null;
    },
  };
}
