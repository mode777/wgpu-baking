import { expect, test } from '@playwright/test';

// docs/TESTING.md e2e budget, spec 1 of 3: bake upstream-basic (CI profile) in the worker on WebGPU; every
// expect.json check holds; cancel mid-bake, then bake again.
const software = process.env.E2E_ALLOW_FALLBACK === '1';

test('bakes upstream-basic in the worker, cancels, and rebakes', async ({ page }) => {
  page.on('console', (m) => {
    if (m.type() === 'error' || m.text().startsWith('lightbaker:')) console.log(`[page] ${m.text()}`);
  });
  await page.goto(`/?profile=ci${software ? '&allowFallbackAdapter=1' : ''}`);

  // capabilities: an adapter, labelled when it is a software one
  await expect(page.locator('body')).toHaveAttribute('data-state', 'ready');
  const adapter = await page.getByTestId('adapter').textContent();
  console.log(`adapter: ${adapter}`);
  if (process.env.E2E_EXPECT_ADAPTER) expect(adapter).toMatch(new RegExp(process.env.E2E_EXPECT_ADAPTER, 'i'));
  await expect(page.getByTestId('software')).toHaveCount(software ? 1 : 0);

  // bake: progress, then every check of expect.json (evaluated by the engine in the worker)
  await page.getByTestId('bake').click();
  await expect(page.getByTestId('progress')).toContainText(/layout|direct|bounce|volume/);
  await expect(page.locator('body')).toHaveAttribute('data-state', /^(done|checks-failed|error-.*)$/, { timeout: 150_000 });
  const outcomes = await page.locator('#checks tr').allTextContents();
  console.log(`checks:\n  ${outcomes.join('\n  ')}`);
  await expect(page.getByTestId('checks-summary')).toHaveAttribute('data-passed', 'true');
  await expect(page.getByTestId('checks-summary')).toHaveAttribute('data-count', '19');
  await expect(page.getByTestId('outcome')).toContainText(software ? 'SOFTWARE ADAPTER' : 'baked');
  console.log(`timings: ${(await page.getByTestId('timings').innerText()).replace(/\s+/g, ' ')}`);

  // cancel a full bake once it is under way
  await page.getByTestId('reuse').uncheck();
  await page.getByTestId('bake').click();
  await expect(page.getByTestId('progress')).toContainText(/^(layout|direct|bounce|volume)/);
  await page.getByTestId('cancel').click({ timeout: 5_000 }).catch(() => console.log('the bake finished before Cancel could be pressed'));
  await expect(page.locator('body')).toHaveAttribute('data-state', /^(cancelled|done)$/, { timeout: 150_000 });
  const afterCancel = await page.locator('body').getAttribute('data-state');
  console.log(`after cancel: ${afterCancel}`);
  if (afterCancel === 'cancelled') await expect(page.getByTestId('cancelled')).toBeVisible();

  // and bake again
  await page.getByTestId('bake').click();
  await expect(page.locator('body')).toHaveAttribute('data-state', /^(done|checks-failed|error-.*)$/, { timeout: 150_000 });
  await expect(page.getByTestId('checks-summary')).toHaveAttribute('data-passed', 'true');
});
