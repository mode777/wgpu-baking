/**
 * The Phase 2 proof of concept: bake `test-scenes/upstream-basic` in a worker on WebGPU and show what
 * happened. URL parameters: `allowFallbackAdapter=1` (bake on a software adapter, labelled), `profile=ci`
 * (a smaller bake for software adapters), `autobake=1`, `debug=1` (a button that destroys the GPU device).
 */

import sceneUrl from '../../test-scenes/upstream-basic/upstream-basic.lbscene?url';
import expectJson from '../../test-scenes/upstream-basic/expect.json?raw';
import type { BakeFailure, ProgressEvent } from './bake/client';
import { BakerService } from './bake/service';
import type { AdapterInfo, BakeConfigOverrides, BakeOutput } from './bake/protocol';
import { readScene } from '@orbitoon/lightbaker';
import { LightmapView, type ViewMode } from './view/lightmapView';

/** Bake settings by profile; `full` is the scene's own (what the native baseline baked). */
export const PROFILES: Record<string, BakeConfigOverrides> = {
  full: {},
  // CI on lavapipe: fewer rays, same layout, so every check of expect.json still holds (P2.6)
  ci: { sampling: { sunSamples: 4, aoRays: 32, giRays: 64, volumeRays: 32 } },
  // the P1.5 baseline's dense variant (4x the texels, 512 rays, 3 bounces): native vs browser on equal work
  dense: { lightmap: { texelsPerMeter: 16, atlasMaxSize: 2048 }, sampling: { giRays: 512 }, gi: { bounces: 3 } },
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

const debug = params.get('debug') === '1';
const service = new BakerService(() => new Worker(new URL('./bake/baker.worker.ts', import.meta.url), { type: 'module' }), allowFallbackAdapter);
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

/** The page follows the baker's state: what it says, which buttons work. */
function render() {
  const s = service;
  ($('recreate') as HTMLButtonElement).hidden = s.state !== 'lost';
  ($('lose') as HTMLButtonElement).hidden = !debug || s.state !== 'ready';
  ($('bake') as HTMLButtonElement).disabled = s.state !== 'ready' || job !== null || !sceneBytes;
  // the unwrap needs the worker, not WebGPU
  ($('unwrap') as HTMLButtonElement).disabled = s.state === 'starting' || s.state === 'lost' || job !== null || !sceneBytes;
  const adapter = s.adapter ?? s.caps?.adapter;
  $('adapter').innerHTML = adapter ? adapterLine(adapter) : '';
  switch (s.state) {
    case 'starting':
      setStatus(s.generation > 1 ? 'recreating the baker…' : 'checking…');
      break;
    case 'unavailable':
      setStatus(`WebGPU baking is unavailable: ${s.caps?.reason ?? s.error?.message ?? 'unknown reason'}`, 'bad');
      document.body.dataset.state = 'no-webgpu';
      break;
    case 'refused':
      setStatus(`Cannot bake: ${s.error?.message}. Add <code>?allowFallbackAdapter=1</code> to bake on it anyway (slow, labelled).`, 'bad');
      document.body.dataset.state = 'fallback-adapter-refused';
      break;
    case 'error':
      setStatus(`Cannot bake: ${s.error?.message ?? 'no adapter'}`, 'bad');
      document.body.dataset.state = s.error?.kind ?? 'error';
      break;
    case 'lost':
      setStatus(`GPU device lost: ${s.error?.message ?? ''}`, 'bad');
      document.body.dataset.state = 'device-lost';
      break;
    case 'ready':
      setStatus(s.adapter?.software ? 'WebGPU ready — on a software adapter' : 'WebGPU ready', s.adapter?.software ? 'warn' : 'good');
      // after startup or a recreate; a finished bake keeps its outcome (done, cancelled, …)
      if (!job && [undefined, '', 'device-lost'].includes(document.body.dataset.state)) document.body.dataset.state = 'ready';
      break;
  }
}
service.onChange = render;

async function start() {
  const [, scene] = await Promise.all([service.start(), fetch(sceneUrl).then((r) => r.arrayBuffer())]);
  sceneBytes = new Uint8Array(scene);
  view?.setScene(sceneBytes);
  render();
  if (service.state === 'ready' && params.get('autobake') === '1') void bake();
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
  const run = service.client.bake(sceneBytes.buffer.slice(sceneBytes.byteOffset, sceneBytes.byteOffset + sceneBytes.byteLength) as ArrayBuffer, { config, expect: expectJson, onProgress });
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
    ['wasm memory', s.wasmMemoryBytes ? mib(s.wasmMemoryBytes) : '—'],
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
$('unwrap').addEventListener('click', () => void unwrap());

/** xatlas lightmap UVs for each mesh of the scene, in the worker (P3.1 evaluation; not yet used by the bake). */
async function unwrap() {
  if (!sceneBytes || job) return;
  const { header } = readScene(sceneBytes);
  ($('unwrap') as HTMLButtonElement).disabled = true;
  $('unwrap-summary').textContent = 'unwrapping…';
  const run = service.client.unwrap(sceneBytes.slice().buffer, { texelsPerMeter: header.lightmap.texelsPerMeter, padding: header.lightmap.padding });
  job = run.jobId;
  try {
    const r = await run.result;
    const valid = r.meshes.every((m) => m.valid);
    const sum = $('unwrap-summary');
    sum.innerHTML = `<span class="${valid ? 'good' : 'bad'}">${valid ? 'valid UVs' : 'INVALID UVs'}</span> · ${fmtMs(r.elapsedMs)} (xatlas load ${fmtMs(r.loadMs)})`;
    sum.dataset.valid = String(valid);
    $('unwrap-result').innerHTML =
      '<tr><td>mesh</td><td class="num">tris</td><td class="num">charts</td><td class="num">tile</td><td class="num">used</td><td class="num">time</td></tr>' +
      r.meshes.map((m) => `<tr><td class="${m.valid ? 'good' : 'bad'}">${m.name}</td><td class="num">${m.triangles}</td><td class="num">${m.charts}</td><td class="num">${m.width}×${m.height}</td><td class="num">${(m.utilisation * 100).toFixed(0)} %</td><td class="num">${fmtMs(m.ms)}</td></tr>`).join('');
  } catch (e) {
    const f = e as BakeFailure;
    $('unwrap-summary').innerHTML = `<span class="bad">${f.kind}: ${f.message}</span>`;
  } finally {
    job = null;
    render();
  }
}
$('cancel').addEventListener('click', () => {
  if (job) service.client.cancel(job);
});

$('recreate').addEventListener('click', () => void service.recreate());
$('lose').addEventListener('click', () => service.client.debugLoseDevice());

void start();
