import { test, expect } from '@playwright/test';

test('renders the setup controls using icons from the dashboard', async ({ page }) => {
  const remoteIconRequests: string[] = [];
  await page.route(/https:\/\/[^/]+\.iconify\.design\//, (route) => {
    remoteIconRequests.push(route.request().url());
    return route.abort();
  });
  await page.route('http://127.0.0.1:1993/v1/system/setup', (route) =>
    route.fulfill({ json: { welcome_complete: false, sign_up_complete: false } })
  );

  await page.goto('/welcome');
  await expect(page.getByRole('button', { name: 'Continue' }).locator('svg path')).toBeAttached();
  expect(remoteIconRequests).toEqual([]);
});
