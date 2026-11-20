/**
 * The page's side of the baker worker: requests out, events in, as promises and callbacks. Works with any
 * `WorkerLike` (a real `Worker`, or a fake wired to `bakerCore` in tests).
 */

import type { AdapterInfo, BakeConfigOverrides, BakeEvent, BakeOutput, BakeRequest, BakerCapabilities, BakerError, UnwrapConfig, UnwrapResult } from './protocol';

export interface WorkerLike {
  postMessage(msg: BakeRequest, transfer?: Transferable[]): void;
  onmessage: ((e: MessageEvent<BakeEvent>) => void) | null;
  terminate(): void;
}

export type ProgressEvent = Extract<BakeEvent, { type: 'progress' }>;

/** Why a bake did not finish: its error, or `cancelled`. */
export type BakeFailure = BakerError | { kind: 'cancelled'; message: string };

export interface InitResult {
  caps: BakerCapabilities;
  /** the adapter the baker opened, when it could */
  adapter?: AdapterInfo;
  error?: BakerError;
}

export class BakerClient {
  private next = 1;
  // bakes resolve with BakeOutput, unwraps with UnwrapResult (both plus elapsedMs)
  private jobs = new Map<string, { resolve: (r: never) => void; reject: (e: BakeFailure) => void; onProgress?: (p: ProgressEvent) => void }>();
  private initWaiter: { resolve: (r: InitResult) => void; caps?: BakerCapabilities } | null = null;
  /** told about errors outside a bake, such as device loss */
  onError: ((e: BakerError) => void) | null = null;

  constructor(private readonly worker: WorkerLike) {
    worker.onmessage = (e) => this.receive(e.data);
  }

  init(allowFallbackAdapter: boolean): Promise<InitResult> {
    return new Promise((resolve) => {
      this.initWaiter = { resolve };
      this.worker.postMessage({ type: 'init', allowFallbackAdapter });
    });
  }

  bake(scene: ArrayBuffer, options: { config?: BakeConfigOverrides; expect?: string; onProgress?: (p: ProgressEvent) => void } = {}): { jobId: string; result: Promise<BakeOutput & { elapsedMs: number }> } {
    const jobId = `job-${this.next++}`;
    const result = new Promise<BakeOutput & { elapsedMs: number }>((resolve, reject) => {
      this.jobs.set(jobId, { resolve: resolve as (r: never) => void, reject, onProgress: options.onProgress });
    });
    // the page keeps its copy of the scene: send a copy, not the buffer itself
    const copy = scene.slice(0);
    this.worker.postMessage({ type: 'bake', jobId, scene: copy, config: options.config, expect: options.expect }, [copy]);
    return { jobId, result };
  }

  /** xatlas lightmap UVs for each mesh of a scene (one object tile each), computed in the worker. */
  unwrap(scene: ArrayBuffer, config: UnwrapConfig): { jobId: string; result: Promise<UnwrapResult & { elapsedMs: number }> } {
    const jobId = `job-${this.next++}`;
    const result = new Promise<UnwrapResult & { elapsedMs: number }>((resolve, reject) => {
      this.jobs.set(jobId, { resolve: resolve as (r: never) => void, reject });
    });
    const copy = scene.slice(0);
    this.worker.postMessage({ type: 'unwrap', jobId, scene: copy, config }, [copy]);
    return { jobId, result };
  }

  cancel(jobId: string): void {
    this.worker.postMessage({ type: 'cancel', jobId });
  }

  /** Debug sessions only: destroy the worker's GPU device (manual device-loss checks). */
  debugLoseDevice(): void {
    this.worker.postMessage({ type: 'debug-lose-device' });
  }

  dispose(): void {
    this.worker.postMessage({ type: 'dispose' });
    this.worker.terminate();
    for (const [, job] of this.jobs) job.reject({ kind: 'internal', message: 'the baker was disposed' });
    this.jobs.clear();
  }

  private receive(e: BakeEvent): void {
    switch (e.type) {
      case 'capabilities':
        if (this.initWaiter) this.initWaiter.caps = e.caps;
        return;
      case 'ready':
        this.finishInit({ adapter: e.adapter });
        return;
      case 'progress':
        this.jobs.get(e.jobId)?.onProgress?.(e);
        return;
      case 'done':
        (this.jobs.get(e.jobId)?.resolve as ((r: BakeOutput & { elapsedMs: number }) => void) | undefined)?.({ ...e.result, elapsedMs: e.elapsedMs });
        this.jobs.delete(e.jobId);
        return;
      case 'unwrapped':
        (this.jobs.get(e.jobId)?.resolve as ((r: UnwrapResult & { elapsedMs: number }) => void) | undefined)?.({ ...e.result, elapsedMs: e.elapsedMs });
        this.jobs.delete(e.jobId);
        return;
      case 'cancelled':
        this.jobs.get(e.jobId)?.reject({ kind: 'cancelled', message: 'the bake was cancelled' });
        this.jobs.delete(e.jobId);
        return;
      case 'error': {
        const job = e.jobId !== undefined ? this.jobs.get(e.jobId) : undefined;
        if (job && e.jobId !== undefined) {
          job.reject(e.error);
          this.jobs.delete(e.jobId);
        } else if (this.initWaiter) {
          this.finishInit({ error: e.error });
        }
        if (!job) this.onError?.(e.error);
        return;
      }
    }
  }

  private finishInit(r: Omit<InitResult, 'caps'>): void {
    const w = this.initWaiter;
    if (!w) return;
    this.initWaiter = null;
    w.resolve({ caps: w.caps ?? { webgpu: false, reason: 'no capabilities received' }, ...r });
  }
}
