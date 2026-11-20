import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const wasmPkg = fileURLToPath(new URL('../crates/lightbaker-web/pkg/lightbaker_web.js', import.meta.url));

export default defineConfig({
  resolve: { alias: { 'lightbaker-web': wasmPkg } },
  // the test scenes live outside app/
  server: { fs: { allow: ['..'] } },
  // the no-webgpu e2e project loads the page from this name (an insecure context)
  preview: { allowedHosts: ['nowebgpu.test'] },
  worker: { format: 'es' },
  build: { target: 'es2022', chunkSizeWarningLimit: 1500 },
  test: { include: ['test/**/*.test.ts'], environment: 'node' },
});
