import { describe, expect, it } from 'vitest';
import { createBakerCore, transferables, type Bridge, type BridgeBaker, type BridgeProgress } from '../src/bake/bakerCore';
import { BakerClient, type ProgressEvent, type WorkerLike } from '../src/bake/client';
import { toBakerError, type AdapterInfo, type BakeEvent, type BakeOutput, type BakeRequest, type BakerCapabilities } from '../src/bake/protocol';

const adapter: AdapterInfo = {
  name: 'Fake GPU', vendor: 1, device: 2, deviceType: 'IntegratedGpu', backend: 'BrowserWebGpu', driver: '', driverInfo: '',
  software: false, isFallbackAdapter: false, limits: { maxStorageBufferBindingSize: 128 << 20, maxBufferSize: 256 << 20, maxComputeWorkgroupsPerDimension: 65535 },
};

function output(): BakeOutput {
  return {
    lbake: new Uint8Array([1, 2, 3]),
    atlas: { width: 4, height: 2 },
    direct: new Float32Array(16),
    indirect: { width: 2, height: 1, data: new Float32Array(6) },
    validity: new Uint8Array(8),
    stats: { triangles: 2, charts: 1, bake: { tiles: 1, tilesBaked: 1, texelsBaked: 8, chunks: 0, chunksBaked: 0, seconds: 0.01 }, adapter: 'Fake GPU', software: false, transfers: { batches: 3, readbacks: 3, readbackBytes: 48, uploadBytes: 100 }, cacheBytes: 0 },
    adapter,
    warnings: [],
  };
}

/** A bridge whose bakes finish when told to (`step`) and reject when cancelled. */
function fakeBridge(caps: BakerCapabilities = { webgpu: true, adapter }, createError?: unknown) {
  const state = { cancelled: false, lost: null as null | ((r: string) => void), finish: null as null | (() => void), progress: null as null | ((p: BridgeProgress) => void), configs: [] as string[] };
  const baker: BridgeBaker = {
    bake(_scene, config, onProgress) {
      state.configs.push(config);
      state.progress = onProgress;
      state.cancelled = false;
      return new Promise((resolve, reject) => {
        state.finish = () => (state.cancelled ? reject({ kind: 'cancelled', message: 'bake cancelled' }) : resolve(output()));
      });
    },
    cancel: () => {
      state.cancelled = true;
    },
    evaluate: () => ({ passed: true, values: {}, outcomes: [{ name: 'x', a: 1, b: 0, passed: true }] }),
    adapter: () => adapter,
    setDeviceLostCallback: (cb) => {
      state.lost = cb;
    },
  };
  const bridge: Bridge = {
    capabilities: async () => caps,
    create: async () => {
      if (createError) throw createError;
      return baker;
    },
  };
  return { bridge, state };
}

/** The client and the worker core in one process, messages delivered asynchronously like a real worker. */
function connect(bridge: Bridge) {
  const events: BakeEvent[] = [];
  let t = 0;
  const worker: WorkerLike = {
    onmessage: null,
    postMessage: (msg: BakeRequest) => void Promise.resolve().then(() => core.handle(msg)),
    terminate: () => {},
  };
  const core = createBakerCore(bridge, (e) => {
    events.push(e);
    queueMicrotask(() => worker.onmessage?.({ data: e } as MessageEvent<BakeEvent>));
  }, () => (t += 10));
  return { client: new BakerClient(worker), events, core };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

describe('baker worker protocol', () => {
  it('reports capabilities, opens the baker, bakes with progress and checks', async () => {
    const { bridge, state } = fakeBridge();
    const { client } = connect(bridge);
    const init = await client.init(false);
    expect(init.caps.webgpu).toBe(true);
    expect(init.adapter?.name).toBe('Fake GPU');

    const seen: ProgressEvent[] = [];
    const job = client.bake(new ArrayBuffer(8), { config: { sampling: { giRays: 64 } }, expect: '{}', onProgress: (p) => seen.push(p) });
    await tick();
    state.progress!({ phase: 'direct', bounce: 0, done: 5, total: 10 });
    state.progress!({ phase: 'bounce', bounce: 1, done: 10, total: 10 });
    state.finish!();
    const r = await job.result;
    expect(seen.map((p) => [p.phase, p.bounce, p.done, p.total])).toEqual([['direct', 0, 5, 10], ['bounce', 1, 10, 10]]);
    expect(seen[0]!.elapsedMs).toBeGreaterThan(0);
    expect(r.checks?.passed).toBe(true);
    expect(r.elapsedMs).toBeGreaterThan(0);
    expect(JSON.parse(state.configs[0]!)).toEqual({ sampling: { giRays: 64 } });
  });

  it('cancels, then bakes again', async () => {
    const { bridge, state } = fakeBridge();
    const { client } = connect(bridge);
    await client.init(false);
    const first = client.bake(new ArrayBuffer(8));
    await tick();
    client.cancel(first.jobId);
    await tick();
    state.finish!();
    await expect(first.result).rejects.toEqual({ kind: 'cancelled', message: 'the bake was cancelled' });
    const second = client.bake(new ArrayBuffer(8));
    await tick();
    state.finish!();
    await expect(second.result).resolves.toMatchObject({ atlas: { width: 4, height: 2 } });
  });

  it('rejects a second bake while one runs', async () => {
    const { bridge, state } = fakeBridge();
    const { client } = connect(bridge);
    await client.init(false);
    const a = client.bake(new ArrayBuffer(8));
    const b = client.bake(new ArrayBuffer(8));
    await expect(b.result).rejects.toMatchObject({ kind: 'internal' });
    state.finish!();
    await expect(a.result).resolves.toBeDefined();
  });

  it('says why WebGPU is unavailable', async () => {
    const { bridge } = fakeBridge({ webgpu: false, reason: 'navigator.gpu is undefined' });
    const { client } = connect(bridge);
    const init = await client.init(false);
    expect(init.caps).toEqual({ webgpu: false, reason: 'navigator.gpu is undefined' });
    expect(init.error).toEqual({ kind: 'no-webgpu', message: 'navigator.gpu is undefined' });
    await expect(client.bake(new ArrayBuffer(8)).result).rejects.toMatchObject({ kind: 'internal' });
  });

  it('passes a refused software adapter on as its kind', async () => {
    const { bridge } = fakeBridge(undefined, { kind: 'fallback-adapter-refused', message: 'the only GPU adapter is a software one (llvmpipe)' });
    const { client } = connect(bridge);
    const init = await client.init(false);
    expect(init.error?.kind).toBe('fallback-adapter-refused');
    expect(init.adapter).toBeUndefined();
  });

  it('maps bridge rejections to error kinds', async () => {
    const { bridge, state } = fakeBridge();
    bridge.create = async () => ({ ...(await fakeBridge().bridge.create(false)), bake: async () => Promise.reject({ kind: 'invalid-scene', message: 'not a lightbaker scene' }) });
    const { client } = connect(bridge);
    await client.init(false);
    await expect(client.bake(new ArrayBuffer(8)).result).rejects.toEqual({ kind: 'invalid-scene', message: 'not a lightbaker scene' });
    expect(state.configs).toEqual([]);
  });

  it('reports device loss and refuses further bakes', async () => {
    const { bridge, state } = fakeBridge();
    const { client } = connect(bridge);
    const errors: string[] = [];
    client.onError = (e) => errors.push(e.kind);
    await client.init(false);
    state.lost!('Destroyed: device.destroy()');
    await tick();
    expect(errors).toEqual(['device-lost']);
    await expect(client.bake(new ArrayBuffer(8)).result).rejects.toMatchObject({ kind: 'internal' });
  });
});

describe('protocol helpers', () => {
  it('normalises thrown values', () => {
    expect(toBakerError({ kind: 'device-lost', message: 'gone' })).toEqual({ kind: 'device-lost', message: 'gone' });
    expect(toBakerError({ kind: 'cancelled', message: 'x' }).kind).toBe('cancelled');
    expect(toBakerError(new Error('boom'))).toEqual({ kind: 'internal', message: 'boom' });
    expect(toBakerError({ kind: 'weird', message: 'x' }).kind).toBe('internal');
  });

  it('transfers every result buffer once', () => {
    const o = output();
    expect(transferables(o)).toHaveLength(4);
  });
});
