// Manual real-GPU check for phase reports (docs/TESTING.md): serves the built app, opens it in an installed
// Chrome/Edge (headless by default) with WebGPU, bakes upstream-basic, then cancels a bake and rebakes, and
// prints the adapter, timings and check results as JSON. Not part of CI (CI uses the Playwright e2e specs).
//
//   pnpm -C app build && node scripts/hw-check.mjs [--channel chrome|msedge] [--headed] [--profile full|ci]
//                                                   [--allow-fallback] [--out file.json] [--screenshot file.png]
//                                                   [--browser-arg <chrome flag>]… (e.g. --use-webgpu-adapter=swiftshader)
//                                                   [--lose-device] (device.destroy() → recreate → bake)
//                                                   [--bench N] (N warm full bakes: median per phase)

import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { chromium } from '@playwright/test';

const arg = (name, fallback) => {
  const i = process.argv.indexOf(name);
  return i < 0 ? fallback : process.argv[i + 1];
};
const has = (name) => process.argv.includes(name);
const channel = arg('--channel', 'chrome');
const profile = arg('--profile', 'full');
const port = 4179;

const server = spawn(process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm', ['exec', 'vite', 'preview', '--port', String(port), '--strictPort'], { cwd: new URL('..', import.meta.url), shell: process.platform === 'win32', stdio: 'ignore' });
let exited = null;
server.on('exit', (c) => (exited = c));
for (let i = 0; ; i++) {
  if (exited !== null) throw new Error(`vite preview exited ${exited} (port ${port} in use?)`);
  if (await fetch(`http://localhost:${port}/`).then((r) => r.ok, () => false)) break;
  if (i > 100) throw new Error('vite preview did not start');
  await new Promise((r) => setTimeout(r, 200));
}

const browser = await chromium.launch({ channel, headless: !has('--headed'), args: ['--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--disable-gpu-watchdog', ...process.argv.flatMap((a, i) => (process.argv[i - 1] === '--browser-arg' ? [a] : []))] });
const result = { channel, profile, browser: browser.version() };
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.on('console', (m) => m.type() === 'error' && console.error('[page]', m.text()));
  await page.goto(`http://localhost:${port}/?profile=${profile}`);
  result.gpuAdapter = await page.evaluate(async () => {
    const a = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance' });
    if (!a) return null;
    const i = a.info ?? {};
    return { vendor: i.vendor, architecture: i.architecture, device: i.device, description: i.description, isFallbackAdapter: i.isFallbackAdapter ?? a.isFallbackAdapter };
  });
  const q = new URLSearchParams({ profile, autobake: '1', ...(has('--allow-fallback') ? { allowFallbackAdapter: '1' } : {}) });
  const t0 = Date.now();
  await page.goto(`http://localhost:${port}/?${q}`);
  await page.waitForFunction(() => !['', 'baking', 'ready', undefined].includes(document.body.dataset.state), null, { timeout: 300_000 });
  result.firstBake = {
    state: await page.evaluate(() => document.body.dataset.state),
    wallMs: Date.now() - t0,
    status: await page.textContent('#status'),
    adapter: await page.textContent('#adapter'),
    outcome: await page.textContent('#outcome'),
    timings: await page.$$eval('#timings tr', (rows) => Object.fromEntries(rows.map((r) => [r.cells[0].textContent, r.cells[1].textContent]))),
    checks: await page.$eval('#checks-summary', (e) => ({ text: e.textContent, passed: e.dataset.passed, count: e.dataset.count })).catch(() => null),
    checkValues: await page.$$eval('#checks tr', (rows) => rows.map((r) => ({ name: r.cells[1].textContent, a: Number(r.cells[2].textContent), b: Number(r.cells[3].textContent), passed: r.cells[0].textContent === '✓' }))),
    warnings: await page.textContent('#warnings'),
  };
  if (result.firstBake.state === 'done') {
    // cancel a full bake (no tile reuse) once it is under way, then bake again
    await page.uncheck('#reuse');
    await page.click('#bake');
    await page.waitForFunction(() => /^(direct|bounce|volume)/.test(document.querySelector('#progress')?.textContent ?? ''), null, { timeout: 60_000 });
    await page.click('#cancel', { timeout: 5_000 }).catch(() => {});
    await page.waitForFunction(() => ['cancelled', 'done'].includes(document.body.dataset.state), null, { timeout: 60_000 });
    result.cancel = await page.evaluate(() => document.body.dataset.state);
    await page.click('#bake');
    await page.waitForFunction(() => !['baking', 'cancelled'].includes(document.body.dataset.state), null, { timeout: 300_000 });
    result.rebake = { state: await page.evaluate(() => document.body.dataset.state), checks: await page.textContent('#checks-summary') };
  }
  await page.screenshot({ path: arg('--screenshot', 'hw-check.png') });
  const bench = Number(arg('--bench', '0'));
  if (bench > 0 && result.firstBake.state === 'done') {
    // warm, full bakes (no tile reuse): the median per phase, comparable with the native `baseline` example
    await page.uncheck('#reuse');
    const runs = [];
    for (let i = 0; i < bench; i++) {
      await page.click('#bake');
      await page.waitForFunction(() => document.body.dataset.state !== 'baking', null, { timeout: 300_000 });
      runs.push(await page.$$eval('#timings tr', (rows) => Object.fromEntries(rows.map((r) => [r.cells[0].textContent, r.cells[1].textContent]))));
    }
    const ms = (v) => (v?.endsWith(' ms') ? parseFloat(v) : v?.endsWith(' s') ? parseFloat(v) * 1000 : NaN);
    const median = (xs) => xs.sort((a, b) => a - b)[Math.floor(xs.length / 2)];
    const keys = Object.keys(runs[0]).filter((k) => !Number.isNaN(ms(runs[0][k])));
    result.bench = { runs: bench, medianMs: Object.fromEntries(keys.map((k) => [k, median(runs.map((r) => ms(r[k])))])), last: runs[runs.length - 1] };
  }
  if (has('--lose-device')) {
    // a real device.destroy() in the worker (debug button), then recovery without reloading the page
    await page.goto(`http://localhost:${port}/?${new URLSearchParams({ profile, debug: '1', ...(has('--allow-fallback') ? { allowFallbackAdapter: '1' } : {}) })}`);
    await page.waitForFunction(() => document.body.dataset.state === 'ready', null, { timeout: 60_000 });
    const t = Date.now();
    await page.click('#lose');
    await page.waitForFunction(() => document.body.dataset.state === 'device-lost', null, { timeout: 30_000 });
    const lost = { afterMs: Date.now() - t, status: await page.textContent('#status'), bakeDisabled: await page.isDisabled('#bake') };
    await page.click('#recreate');
    await page.waitForFunction(() => document.body.dataset.state === 'ready', null, { timeout: 60_000 });
    await page.click('#bake');
    await page.waitForFunction(() => !['ready', 'baking'].includes(document.body.dataset.state), null, { timeout: 300_000 });
    result.deviceLoss = { lost, recreated: true, bakeAfterRecreate: await page.evaluate(() => document.body.dataset.state), checks: await page.textContent('#checks-summary') };
  }
} finally {
  await browser.close();
  // on Windows the server runs under a shell: end its whole process tree
  if (process.platform === 'win32') spawn('taskkill', ['/pid', String(server.pid), '/T', '/F'], { stdio: 'ignore' });
  else server.kill();
}
const json = JSON.stringify(result, null, 2);
console.log(json);
const out = arg('--out');
if (out) writeFileSync(out, json + '\n');
process.exit(result.firstBake?.state === 'done' && result.rebake?.state === 'done' && (!result.deviceLoss || result.deviceLoss.bakeAfterRecreate === 'done') ? 0 : 1);
