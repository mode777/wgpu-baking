/**
 * The Phase 2 proof of concept: bake `test-scenes/upstream-basic` in a worker on WebGPU and show what
 * happened. URL parameters: `allowFallbackAdapter=1` (bake on a software adapter, labelled), `profile=ci`
 * (a smaller bake for software adapters), `autobake=1`.
 */

import sceneUrl from '../../test-scenes/upstream-basic/upstream-basic.lbscene?url';
import expectJson from '../../test-scenes/upstream-basic/expect.json?raw';
import { BakerClient, type BakeFailure, type ProgressEvent } from './bake/client';
import type { AdapterInfo, BakeConfigOverrides, BakeOutput } from './bake/protocol';
import { LightmapView, type ViewMode } from './view/lightmapView';

/** Bake settings by profile; `full` is the scene's own (what the native baseline baked). */
export const PROFILES: Record<string, BakeConfigOverrides> = {
  full: {},
  // CI on lavapipe: fewer rays, same layout, so every check of expect.json still holds (P2.6)
  ci: { sampling: { sunSamples: 4, aoRays: 32, giRays: 64, volumeRays: 32 } },
};

const params = new URLSearchParams(location.search);
const allowFallbackAdapter = params.get('allowFallbackAdapter') === '1';
const profile = params.get('profile') ?? 'full';
const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const fmtMs = (ms: number) => (ms < 1000 ? `${ms.toFixed(0)} ms` : `${(ms / 1000).toFixed(2)} s`);
const mib = (b: number) => `${(b / 1048576).toFixed(2)} MiB`;

$('profile').textContent = profile in PROFILES ? profile : `${profile} (unknown: using full)`;
const view = (() => {
  try {
    return new LightmapView($('view'));
  } catch (e) {
    // no WebGL here: the baker does not need it, only the 3D view does
    $('view').insertAdjacentHTML('beforeend', `<p class="warn" data-testid="no-view" style="padding: 40px">3D view unavailable: ${e instanceof Error ? e.message : String(e)}</p>`);
    return null;
  }
})();
document.querySelectorAll<HTMLInputElement>('input[name=mode]').forEach((r) => r.addEventListener('change', () => view?.setMode(r.value as ViewMode)));

const client = new BakerClient(new Worker(new URL('./bake/baker.worker.ts', import.meta.url), { type: 'module' }));
let sceneBytes: Uint8Array | null = null;
let job: string | null = null;

function adapterLine(a: AdapterInfo): string {
  const soft = a.software ? ' <span class="software" data-testid="software">SOFTWARE ADAPTER</span>' : '';
  const l = a.limits;
  return `${a.name} (${a.backend}, ${a.deviceType})${soft}<br><small>storage binding ${Math.floor(l.maxStorageBufferBindingSize / 1048576)} MiB · buffer ${Math.floor(l.maxBufferSize / 1048576)} MiB · workgroups/dim ${l.maxComputeWorkgroupsPerDimension}</small>`;
}

function setStatus(html: string, cls: 'good' | 'bad' | 'warn' | '' = '') {
  const s = $('status');
  s.innerHTML = html;
  s.className = cls;
}

client.onError = (e) => {
  if (e.kind === 'device-lost') {
    setStatus(`GPU device lost: ${e.message}`, 'bad');
    ($('bake') as HTMLButtonElement).disabled = true;
  }
};

async function start() {
  const [init, scene] = await Promise.all([client.init(allowFallbackAdapter), fetch(sceneUrl).then((r) => r.arrayBuffer())]);
  sceneBytes = new Uint8Array(scene);
  view?.setScene(sceneBytes);
  if (!init.caps.webgpu) {
    setStatus(`WebGPU baking is unavailable: ${init.caps.reason ?? 'unknown reason'}`, 'bad');
    document.body.dataset.state = 'no-webgpu';
    return;
  }
  if (init.error || !init.adapter) {
    const why = init.error?.kind === 'fallback-adapter-refused' ? `${init.error.message}. Add <code>?allowFallbackAdapter=1</code> to bake on it anyway (slow, labelled).` : (init.error?.message ?? 'no adapter');
    setStatus(`Cannot bake: ${why}`, 'bad');
    if (init.caps.adapter) $('adapter').innerHTML = adapterLine(init.caps.adapter);
    document.body.dataset.state = init.error?.kind ?? 'error';
    return;
  }
  setStatus(init.adapter.software ? 'WebGPU ready — on a software adapter' : 'WebGPU ready', init.adapter.software ? 'warn' : 'good');
  $('adapter').innerHTML = adapterLine(init.adapter);
  ($('bake') as HTMLButtonElement).disabled = false;
  document.body.dataset.state = 'ready';
  if (params.get('autobake') === '1') void bake();
}

async function bake() {
  if (!sceneBytes || job) return;
  ($('bake') as HTMLButtonElement).disabled = true;
  ($('cancel') as HTMLButtonElement).disabled = false;
  $('outcome').textContent = '';
  $('progress').textContent = 'starting…';
  document.body.dataset.state = 'baking';
  const phaseEnds: [string, number][] = [];
  let lastPhase = '';
  const onProgress = (p: ProgressEvent) => {
    const name = p.phase === 'bounce' ? `bounce ${p.bounce}` : p.phase;
    if (name !== lastPhase) phaseEnds.push([name, p.elapsedMs]);
    phaseEnds[phaseEnds.length - 1]![1] = p.elapsedMs;
    lastPhase = name;
    const bar = $('bar') as HTMLProgressElement;
    bar.max = Math.max(p.total, 1);
    bar.value = p.done;
    $('progress').textContent = `${name}: ${p.done} / ${p.total} · ${fmtMs(p.elapsedMs)}`;
  };
  const config = { ...(PROFILES[profile] ?? {}), incremental: { reuseTiles: ($('reuse') as HTMLInputElement).checked } };
  const run = client.bake(sceneBytes.buffer.slice(sceneBytes.byteOffset, sceneBytes.byteOffset + sceneBytes.byteLength) as ArrayBuffer, { config, expect: expectJson, onProgress });
  job = run.jobId;
  try {
    const out = await run.result;
    showResult(out, phaseEnds);
    await view?.showBake(sceneBytes, out);
    document.body.dataset.state = out.checks?.passed ? 'done' : 'checks-failed';
  } catch (e) {
    const f = e as BakeFailure;
    $('outcome').innerHTML = f.kind === 'cancelled' ? '<span class="warn" data-testid="cancelled">cancelled</span>' : `<span class="bad">${f.kind}: ${f.message}</span>`;
    document.body.dataset.state = f.kind === 'cancelled' ? 'cancelled' : `error-${f.kind}`;
  } finally {
    job = null;
    ($('cancel') as HTMLButtonElement).disabled = true;
    ($('bake') as HTMLButtonElement).disabled = document.body.dataset.state === 'error-device-lost';
  }
}

function showResult(out: BakeOutput & { elapsedMs: number }, phaseEnds: [string, number][]) {
  const s = out.stats, t = s.transfers;
  const label = s.software ? ' <span class="software">SOFTWARE ADAPTER</span>' : '';
  $('outcome').innerHTML = `<span class="good" data-testid="done">baked</span> in ${fmtMs(out.elapsedMs)}${label}`;
  let prev = 0;
  const rows: [string, string][] = phaseEnds.map(([name, end]) => {
    const r: [string, string] = [name, fmtMs(end - prev)];
    prev = end;
    return r;
  });
  rows.push(
    ['total (worker)', fmtMs(out.elapsedMs)],
    ['engine bake.seconds', `${s.bake.seconds.toFixed(3)} s`],
    ['triangles / charts', `${s.triangles} / ${s.charts}`],
    ['tiles baked', `${s.bake.tilesBaked} of ${s.bake.tiles}`],
    ['atlas', `${out.atlas.width}×${out.atlas.height} (indirect ${out.indirect.width}×${out.indirect.height})`],
    ['GPU batches / readbacks', `${t.batches} / ${t.readbacks}`],
    ['read back / uploaded', `${mib(t.readbackBytes)} / ${mib(t.uploadBytes)}`],
    ['.lbake', mib(out.lbake.byteLength)],
  );
  $('timings').innerHTML = rows.map(([k, v]) => `<tr><td>${k}</td><td class="num">${v}</td></tr>`).join('');
  const c = out.checks;
  if (c) {
    const passed = c.outcomes.filter((o) => o.passed).length;
    const sum = $('checks-summary');
    sum.innerHTML = `<span class="${c.passed ? 'good' : 'bad'}">${passed} of ${c.outcomes.length} checks passed</span>`;
    sum.dataset.passed = String(c.passed);
    sum.dataset.count = String(c.outcomes.length);
    $('checks').innerHTML = c.outcomes.map((o) => `<tr><td class="${o.passed ? 'good' : 'bad'}">${o.passed ? '✓' : '✗'}</td><td>${o.name}</td><td class="num">${o.a.toFixed(3)}</td><td class="num">${o.b.toFixed(3)}</td></tr>`).join('');
  }
  $('warnings').textContent = out.warnings.length ? out.warnings.join('\n') : '—';
}

$('bake').addEventListener('click', () => void bake());
$('cancel').addEventListener('click', () => {
  if (job) client.cancel(job);
});

void start();
