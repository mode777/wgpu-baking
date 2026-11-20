// Manual real-GPU check for phase reports (docs/TESTING.md): serves the built app, opens it in an installed
// Chrome/Edge (headless by default) with WebGPU, bakes upstream-basic, then cancels a bake and rebakes, and
// prints the adapter, timings and check results as JSON. Not part of CI (CI uses the Playwright e2e specs).
//
//   pnpm -C app build && node scripts/hw-check.mjs [--channel chrome|msedge] [--headed] [--profile full|ci]
//                                                   [--allow-fallback] [--out file.json] [--screenshot file.png]
//                                                   [--browser-arg <chrome flag>]… (e.g. --use-webgpu-adapter=swiftshader)

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
process.exit(result.firstBake?.state === 'done' && result.rebake?.state === 'done' ? 0 : 1);
