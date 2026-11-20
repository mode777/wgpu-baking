/**
 * The page's baker: one worker at a time, made by a factory so a lost GPU device can be recovered from by
 * starting a fresh worker (new device, empty tile store) without reloading the page (ADR-0002).
 */

import { BakerClient, type InitResult, type WorkerLike } from './client';
import type { AdapterInfo, BakerCapabilities, BakerError } from './protocol';

export type BakerState =
  /** opening the worker and its device */
  | 'starting'
  | 'ready'
  /** no WebGPU here */
  | 'unavailable'
  /** only a software adapter, not allowed */
  | 'refused'
  /** another failure while opening */
  | 'error'
  /** the device was lost: recreate */
  | 'lost';

export class BakerService {
  state: BakerState = 'starting';
  caps: BakerCapabilities | null = null;
  adapter: AdapterInfo | null = null;
  error: BakerError | null = null;
  /** generation of the worker in use (1 for the first) */
  generation = 0;
  /** told after every state change */
  onChange: (() => void) | null = null;
  private current: BakerClient | null = null;

  constructor(
    private readonly createWorker: () => WorkerLike,
    private readonly allowFallbackAdapter: boolean,
  ) {}

  get client(): BakerClient {
    if (!this.current) throw new Error('the baker has not been started');
    return this.current;
  }

  async start(): Promise<InitResult> {
    this.generation++;
    const client = new BakerClient(this.createWorker());
    this.current = client;
    this.set('starting', null);
    client.onError = (e) => {
      if (e.kind === 'device-lost' && client === this.current && this.state !== 'lost') this.set('lost', e);
    };
    const r = await client.init(this.allowFallbackAdapter);
    if (client !== this.current) return r;
    this.caps = r.caps;
    this.adapter = r.adapter ?? null;
    if (!r.caps.webgpu) this.set('unavailable', r.error ?? { kind: 'no-webgpu', message: r.caps.reason ?? 'WebGPU is unavailable' });
    else if (r.error?.kind === 'fallback-adapter-refused') this.set('refused', r.error);
    else if (r.error || !r.adapter) this.set('error', r.error ?? { kind: 'internal', message: 'no adapter' });
    else this.set('ready', null);
    return r;
  }

  /** A fresh worker with a new device; the old one (and its baked tiles) is discarded. */
  async recreate(): Promise<InitResult> {
    this.current?.dispose();
    this.current = null;
    return this.start();
  }

  private set(state: BakerState, error: BakerError | null): void {
    this.state = state;
    this.error = error;
    this.onChange?.();
  }
}
