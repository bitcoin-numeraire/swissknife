import { test, expect } from '@playwright/test';

test('signs in with a local mock OAuth2 persona', async ({ page, request }) => {
  const tokenResponse = await request.post('http://127.0.0.1:8090/default/token', {
    form: {
      grant_type: 'client_credentials',
      client_id: 'dev-readonly',
      client_secret: 'dev-secret',
      scope: 'openid',
    },
  });

  expect(tokenResponse.ok()).toBe(true);
  await expect((await tokenResponse.json()).access_token).toMatch(/\S+\.\S+\.\S+/);

  await page.route('http://127.0.0.1:1993/v1/system/setup', async (route) => {
    await route.fulfill({
      contentType: 'application/json',
      json: {
        sign_up_complete: true,
        welcome_complete: true,
      },
    });
  });

  await page.goto('/login');
  await expect(page.getByTestId('mock-oauth2-persona-dev-admin')).toBeVisible();

  await page.getByTestId('mock-oauth2-persona-dev-readonly').click();

  await expect
    .poll(() => page.evaluate(() => sessionStorage.getItem('jwt_access_token')))
    .toMatch(/\S+\.\S+\.\S+/);
});
