import { defineConfig } from '@playwright/test';

/**
 * Browser e2e (docs/TESTING.md: at most three specs). Serves the production build. Environment:
 * - E2E_CHANNEL=chrome|msedge — an installed browser instead of Playwright's Chromium (local hardware runs)
 * - E2E_HEADED=1             — headed (CI runs it under xvfb-run: what WebGPU on lavapipe needs)
 * - E2E_ALLOW_FALLBACK=1     — bake on a software adapter (CI: lavapipe), results labelled
 * - E2E_BROWSER_ARGS='…'    — extra browser flags, e.g. --use-webgpu-adapter=swiftshader (software path locally)
 * - E2E_EXPECT_ADAPTER=re    — the adapter line must match (CI: a software adapter; Chromium offers SwiftShader there, ADR-0009)
 * The server listens on 127.0.0.1 (a secure context, like localhost; Linux resolves localhost to ::1).
 */
const port = 4180;
const channel = process.env.E2E_CHANNEL || undefined;
const headless = process.env.E2E_HEADED !== '1';
const linux = process.platform === 'linux';
// three.js' CI flags for WebGPU on Mesa lavapipe (ADR-0009)
const webgpuArgs = ['--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--disable-gpu-watchdog', ...(linux ? ['--enable-features=Vulkan', '--disable-vulkan-surface', '--disable-gpu-driver-bug-workarounds', '--no-sandbox'] : []), ...(process.env.E2E_BROWSER_ARGS?.split(' ').filter(Boolean) ?? [])];

export default defineConfig({
  testDir: 'e2e',
  timeout: 180_000,
  expect: { timeout: 30_000 },
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: { baseURL: `http://127.0.0.1:${port}`, trace: 'retain-on-failure' },
  webServer: { command: `pnpm exec vite preview --host 127.0.0.1 --port ${port} --strictPort`, url: `http://127.0.0.1:${port}`, reuseExistingServer: !process.env.CI, timeout: 60_000 },
  projects: [
    { name: 'webgpu', testMatch: 'bake-poc.spec.ts', use: { channel, headless, launchOptions: { args: webgpuArgs } } },
    // WebGPU really absent: browsers expose navigator.gpu only in secure contexts, so the page is loaded over
    // plain http from a name that is not localhost (as a site served over http would be)
    { name: 'no-webgpu', testMatch: 'no-webgpu.spec.ts', use: { channel, headless, baseURL: `http://nowebgpu.test:${port}`, launchOptions: { args: ['--host-resolver-rules=MAP nowebgpu.test 127.0.0.1', ...(linux ? ['--no-sandbox'] : [])] } } },
  ],
});
