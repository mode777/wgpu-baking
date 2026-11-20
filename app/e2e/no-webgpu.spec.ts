import { expect, test } from '@playwright/test';

// docs/TESTING.md e2e budget, spec 2 of 3: without WebGPU the page says why and cannot bake. The page is
// loaded in an insecure context (playwright.config.ts), where browsers do not expose navigator.gpu.

test('explains that WebGPU is unavailable and disables baking', async ({ page }) => {
  page.on('console', (m) => console.log(`[page] ${m.type()}: ${m.text()}`));
  page.on('pageerror', (e) => console.log(`[pageerror] ${e.message}`));
  await page.goto('/');
  expect(await page.evaluate(() => [window.isSecureContext, 'gpu' in navigator])).toEqual([false, false]);
  await expect(page.locator('body')).toHaveAttribute('data-state', 'no-webgpu');
  await expect(page.getByTestId('status')).toContainText('WebGPU baking is unavailable');
  await expect(page.getByTestId('status')).toContainText('navigator.gpu');
  await expect(page.getByTestId('bake')).toBeDisabled();
});
