/// <reference lib="webworker" />
/**
 * The baker worker (ADR-0002): the Rust baker compiled to wasm, on its own WebGPU device, driven by
 * `BakeRequest` messages. All logic is in `bakerCore.ts`; this file only wires it to `lightbaker-web`.
 */

import init, { WebBaker, capabilities } from 'lightbaker-web';
import { createBakerCore, type Bridge, type BridgeBaker } from './bakerCore';
import type { BakeRequest } from './protocol';

declare const self: DedicatedWorkerGlobalScope;

const ready = init();

const bridge: Bridge = {
  async capabilities() {
    await ready;
    return capabilities();
  },
  async create(allowFallbackAdapter) {
    await ready;
    const b = await WebBaker.create(allowFallbackAdapter);
    const baker: BridgeBaker = {
      bake: (scene, config, onProgress) => b.bake(scene, config, onProgress),
      cancel: () => b.cancel(),
      evaluate: (expect) => b.evaluate(expect),
      adapter: () => b.adapter(),
      setDeviceLostCallback: (cb) => b.setDeviceLostCallback(cb),
      free: () => b.free(),
    };
    return baker;
  },
};

const core = createBakerCore(bridge, (event, transfer) => self.postMessage(event, { transfer: transfer ?? [] }));

self.onmessage = (e: MessageEvent<BakeRequest>) => {
  void core.handle(e.data);
};
