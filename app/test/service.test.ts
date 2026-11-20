import { describe, expect, it } from 'vitest';
import { createBakerCore, type Bridge, type BridgeBaker, type BridgeProgress } from '../src/bake/bakerCore';
import type { WorkerLike } from '../src/bake/client';
import type { AdapterInfo, BakeEvent, BakeOutput, BakeRequest } from '../src/bake/protocol';
import { BakerService } from '../src/bake/service';

const adapter: AdapterInfo = {
  name: 'Fake GPU', vendor: 0, device: 0, deviceType: 'Other', backend: 'BrowserWebGpu', driver: '', driverInfo: '',
  software: false, isFallbackAdapter: false, limits: { maxStorageBufferBindingSize: 1, maxBufferSize: 1, maxComputeWorkgroupsPerDimension: 1 },
};

const output = (): BakeOutput => ({
  lbake: new Uint8Array(1), atlas: { width: 1, height: 1 }, direct: new Float32Array(2), indirect: { width: 1, height: 1, data: new Float32Array(3) }, validity: new Uint8Array(1),
  stats: { triangles: 0, charts: 0, bake: { tiles: 1, tilesBaked: 1, texelsBaked: 1, chunks: 0, chunksBaked: 0, seconds: 0 }, adapter: 'Fake GPU', software: false, transfers: { batches: 1, readbacks: 1, readbackBytes: 16, uploadBytes: 16 }, cacheBytes: 0 },
  adapter, warnings: [],
});

/**
 * A fake GPU device as the bridge sees it (the mocked `GPUDevice.lost`): `destroy()` resolves `lost`,
 * after which bakes reject with `device-lost`, as the real bridge does.
 */
function fakeDevice() {
  let lostCb: ((r: string) => void) | null = null;
  let lost = false;
  let pending: { reject: (e: unknown) => void; resolve: (o: BakeOutput) => void } | null = null;
  const baker: BridgeBaker = {
    bake: (_s, _c, onProgress: (p: BridgeProgress) => void) => {
      if (lost) return Promise.reject({ kind: 'device-lost', message: 'the GPU device was lost (Destroyed): create a new baker' });
      onProgress({ phase: 'direct', bounce: 0, done: 0, total: 1 });
      return new Promise((resolve, reject) => (pending = { resolve, reject }));
    },
    cancel: () => {},
    evaluate: () => ({ passed: true, values: {}, outcomes: [] }),
    adapter: () => adapter,
    setDeviceLostCallback: (cb) => (lostCb = cb),
    loseDevice: () => {
      lost = true;
      // the browser resolves `device.lost` asynchronously, and the awaited readback then fails
      queueMicrotask(() => {
        lostCb?.('Destroyed: device.destroy()');
        pending?.reject({ kind: 'device-lost', message: 'the GPU device was lost (Destroyed)' });
        pending = null;
      });
    },
  };
  const bridge: Bridge = { capabilities: async () => ({ webgpu: true, adapter }), create: async () => baker };
  return { bridge, finish: () => pending?.resolve(output()) };
}

/** A worker factory: each worker gets its own core and device. */
function workers() {
  const devices: ReturnType<typeof fakeDevice>[] = [];
  const terminated: number[] = [];
  const create = (): WorkerLike => {
    const device = fakeDevice();
    const index = devices.push(device) - 1;
    const worker: WorkerLike = {
      onmessage: null,
      postMessage: (msg: BakeRequest) => void Promise.resolve().then(() => core.handle(msg)),
      terminate: () => terminated.push(index),
    };
    const core = createBakerCore(device.bridge, (e) => queueMicrotask(() => worker.onmessage?.({ data: e } as MessageEvent<BakeEvent>)));
    return worker;
  };
  return { create, devices, terminated };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

describe('device loss', () => {
  it('outside a bake: lost, then recreated with a new worker that bakes', async () => {
    const w = workers();
    const s = new BakerService(w.create, false);
    const states: string[] = [];
    s.onChange = () => states.push(s.state);
    await s.start();
    expect(s.state).toBe('ready');

    s.client.debugLoseDevice();
    await tick();
    expect(s.state).toBe('lost');
    expect(s.error?.kind).toBe('device-lost');
    await expect(s.client.bake(new ArrayBuffer(4)).result).rejects.toMatchObject({ kind: 'internal' });

    await s.recreate();
    expect(s.state).toBe('ready');
    expect(s.generation).toBe(2);
    expect(w.terminated).toEqual([0]);
    const job = s.client.bake(new ArrayBuffer(4));
    await tick();
    w.devices[1]!.finish();
    await expect(job.result).resolves.toMatchObject({ atlas: { width: 1, height: 1 } });
    expect(states).toEqual(['starting', 'ready', 'lost', 'starting', 'ready']);
  });

  it('during a bake: the bake ends with device-lost, reported once', async () => {
    const w = workers();
    const s = new BakerService(w.create, false);
    let lostReports = 0;
    s.onChange = () => s.state === 'lost' && lostReports++;
    await s.start();
    const job = s.client.bake(new ArrayBuffer(4));
    await tick();
    s.client.debugLoseDevice();
    await expect(job.result).rejects.toMatchObject({ kind: 'device-lost' });
    await tick();
    expect(s.state).toBe('lost');
    expect(lostReports).toBe(1);
  });

  it('a refused software adapter is not a loss', async () => {
    const s = new BakerService(() => {
      const worker: WorkerLike = { onmessage: null, postMessage: (m: BakeRequest) => void Promise.resolve().then(() => core.handle(m)), terminate: () => {} };
      const core = createBakerCore({ capabilities: async () => ({ webgpu: true }), create: async () => Promise.reject({ kind: 'fallback-adapter-refused', message: 'llvmpipe' }) }, (e) => queueMicrotask(() => worker.onmessage?.({ data: e } as MessageEvent<BakeEvent>)));
      return worker;
    }, false);
    await s.start();
    expect(s.state).toBe('refused');
  });
});
