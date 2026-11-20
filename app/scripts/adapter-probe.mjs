// Diagnostics: which WebGPU adapter Chromium exposes under different flag sets (e.g. lavapipe vs SwiftShader on
// a GPU-less Linux runner). Prints one JSON line per flag set. Not a test.
//
//   node scripts/adapter-probe.mjs [--channel chrome] [--headed]

import { createServer } from 'node:http';
import { chromium } from '@playwright/test';

// requestAdapter needs a secure context: a page from 127.0.0.1
const server = createServer((_, res) => res.end('<p>probe</p>')).listen(0, '127.0.0.1');
await new Promise((r) => server.once('listening', r));
const url = `http://127.0.0.1:${server.address().port}/`;

const channel = process.argv.includes('--channel') ? process.argv[process.argv.indexOf('--channel') + 1] : undefined;
const headless = !process.argv.includes('--headed');
const base = ['--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--disable-gpu-watchdog', ...(process.platform === 'linux' ? ['--no-sandbox'] : [])];
const sets = {
  base,
  vulkan: [...base, '--enable-features=Vulkan', '--disable-vulkan-surface', '--disable-gpu-driver-bug-workarounds'],
  'vulkan+native': [...base, '--enable-features=Vulkan', '--disable-vulkan-surface', '--use-vulkan=native'],
  'vulkan+angle': [...base, '--enable-features=Vulkan,VulkanFromANGLE,DefaultANGLEVulkan', '--use-angle=vulkan', '--use-vulkan=native'],
  'adapter=default': [...base, '--enable-features=Vulkan', '--use-webgpu-adapter=default'],
  'no-swiftshader': [...base, '--enable-features=Vulkan', '--disable-software-rasterizer'],
};

for (const [name, args] of Object.entries(sets)) {
  let browser;
  try {
    browser = await chromium.launch({ channel, headless, args });
    const page = await browser.newPage();
    await page.goto(url);
    const r = await page.evaluate(async () => {
      if (!navigator.gpu) return { gpu: false };
      const out = {};
      for (const [k, o] of Object.entries({ default: {}, highPerformance: { powerPreference: 'high-performance' }, fallback: { forceFallbackAdapter: true } })) {
        const a = await navigator.gpu.requestAdapter(o);
        const i = a?.info;
        out[k] = a ? `${i?.vendor}/${i?.architecture}/${i?.description}${(i?.isFallbackAdapter ?? a.isFallbackAdapter) ? ' [fallback]' : ''}` : null;
      }
      return out;
    });
    console.log(JSON.stringify({ set: name, ...r }));
  } catch (e) {
    console.log(JSON.stringify({ set: name, error: String(e).slice(0, 200) }));
  } finally {
    await browser?.close();
  }
}
server.close();
