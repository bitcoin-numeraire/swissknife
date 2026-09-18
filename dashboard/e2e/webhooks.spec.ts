import type { Page, APIRequestContext } from '@playwright/test';

import { createHmac } from 'node:crypto';
import { test, expect } from '@playwright/test';
import { readFile, writeFile } from 'node:fs/promises';

const api = process.env.SWISSKNIFE_E2E_API ?? 'http://127.0.0.1:21993';
const issuer = process.env.SWISSKNIFE_E2E_OAUTH2 ?? 'http://127.0.0.1:8090';
const receiver = process.env.SWISSKNIFE_E2E_RECEIVER_URL;
const receiverFile = process.env.SWISSKNIFE_E2E_RECEIVER_FILE;

async function token(request: APIRequestContext, persona: string) {
  const response = await request.post(`${issuer}/default/token`, {
    form: {
      grant_type: 'client_credentials',
      client_id: persona,
      client_secret: 'itest-secret',
      scope: 'openid',
    },
  });
  expect(response.ok()).toBeTruthy();
  return (await response.json()).access_token as string;
}

async function signIn(page: Page, request: APIRequestContext, persona: string) {
  const accessToken = await token(request, persona);
  await request.post(`${api}/v1/system/mark-welcome-complete`);
  await page.addInitScript((value) => {
    if (!sessionStorage.getItem('webhook_test_initialized')) {
      sessionStorage.setItem('jwt_access_token', value);
      sessionStorage.setItem('webhook_test_initialized', 'true');
    }
    localStorage.setItem('i18nextLng', 'en');
  }, accessToken);
  await page.goto('/developers?tab=webhooks');
  await expect(page.getByRole('heading', { name: 'My webhooks', exact: true })).toBeVisible();
  const account = await request.get(`${api}/v1/me`, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  expect(account.ok()).toBeTruthy();
  return { accessToken, account: await account.json() };
}

async function createWebhook(page: Page, destination: string, walletId?: string) {
  await page.getByRole('button', { name: 'Create webhook', exact: true }).click();
  const dialog = page.getByRole('dialog');
  if (walletId) {
    await dialog.getByLabel('Wallet', { exact: true }).click();
    await page.getByRole('option', { name: new RegExp(walletId) }).click();
  }
  await dialog.getByLabel('Destination URL').fill(destination);
  await dialog.getByRole('checkbox', { name: 'invoice.paid', exact: true }).check();
  await dialog.getByRole('button', { name: 'Create webhook', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Save your signing secret' })).toBeVisible();
  const secret = await page.getByLabel('Signing secret', { exact: true }).inputValue();
  await page.getByRole('checkbox', { name: 'I have saved this secret securely' }).check();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page).toHaveURL(/id=/);
  return secret;
}

async function confirmAction(page: Page, button: string, title: string) {
  await page.getByRole('button', { name: button, exact: true }).click();
  const dialog = page
    .getByRole('dialog')
    .filter({ has: page.getByRole('heading', { name: title, exact: true }) });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Confirm', exact: true }).click();
  await expect(dialog).not.toBeVisible();
}

test('ordinary owner manages the complete lifecycle, diagnostics and old API-key links', async ({
  page,
  request,
}) => {
  const { accessToken } = await signIn(page, request, 'itest-account');
  await expect(page.getByRole('button', { name: 'Instance resources' })).toHaveCount(0);
  const destination = `https://127.0.0.1/browser-${Date.now()}`;
  const secret = await createWebhook(page, destination);
  const originalUrl = page.url();
  await expect(page.getByText(destination, { exact: true })).toBeVisible();
  await expect(page.getByLabel('Signing secret', { exact: true })).toHaveCount(0);
  await page.reload();
  await expect(page.getByLabel('Signing secret', { exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Edit webhook', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Destination URL').fill(`${destination}-edited`);
  await page.getByRole('dialog').getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByText(`${destination}-edited`, { exact: true })).toBeVisible();
  await confirmAction(page, 'Send test event', 'Send test event?');
  const details = page
    .getByRole('dialog')
    .filter({ has: page.getByRole('heading', { name: 'Delivery details' }) });
  await expect(details.getByText('Stopped', { exact: true })).toBeVisible({ timeout: 20_000 });
  const payload = await details.getByLabel('Signed payload').inputValue();
  expect(JSON.parse(payload).type).toBe('webhook.test');
  await details.getByRole('button', { name: 'Retry delivery', exact: true }).click();
  await page
    .getByRole('dialog')
    .filter({ has: page.getByRole('heading', { name: 'Retry this delivery?' }) })
    .getByRole('button', { name: 'Confirm' })
    .click();
  await expect(details.getByText('2 / 8 attempts', { exact: true })).toBeVisible({
    timeout: 20_000,
  });
  await details.getByRole('button', { name: 'Close', exact: true }).click();
  await confirmAction(page, 'Disable', 'Disable webhook?');
  await expect(page.getByRole('button', { name: 'Send test event', exact: true })).toBeDisabled();
  await confirmAction(page, 'Enable', 'Enable webhook?');
  await expect(page.getByRole('button', { name: 'Send test event', exact: true })).toBeEnabled();
  await confirmAction(page, 'Rotate secret', 'Rotate signing secret?');
  const rotated = await page.getByLabel('Signing secret', { exact: true }).inputValue();
  expect(rotated).not.toBe(secret);
  await page.getByRole('checkbox', { name: 'I have saved this secret securely' }).check();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  const storage = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)])
  );
  expect(storage).not.toContain(secret);
  expect(storage).not.toContain(rotated);
  await confirmAction(page, 'Delete', 'Delete webhook?');
  await expect(page).not.toHaveURL(/(?:\?|&)id=/);
  const id = new URL(originalUrl).searchParams.get('id');
  const history = await request.get(`${api}/v1/me/webhooks?ids=${id}`, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  expect(await history.json()).toEqual([]);
  await page.goto('/build/api-keys');
  await expect(page).toHaveURL(/developers.*tab=api-keys/);
  await expect(page.getByRole('tab', { name: 'API Keys', exact: true })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await page.getByRole('button', { name: 'New', exact: true }).click();
  const keyDialog = page.getByRole('dialog');
  const keyName = `Browser key ${Date.now()}`;
  await page.getByLabel('Token name', { exact: true }).fill(keyName);
  await page.getByRole('button', { name: 'Generate token', exact: true }).click();
  await expect(keyDialog.getByText(/This token will never be displayed anymore/)).toBeVisible();
  const key = await page.getByLabel('API token', { exact: true }).inputValue();
  expect((await request.get(`${api}/v1/me`, { headers: { 'api-key': key } })).ok()).toBeTruthy();
  await keyDialog.getByRole('button', { name: 'Close', exact: true }).click();
  const keyRow = page.getByRole('row').filter({ hasText: keyName });
  await keyRow.getByRole('button', { name: 'Scopes', exact: true }).click();
  await expect(page.getByText('Account wallets', { exact: true })).toBeVisible();
  await keyRow.getByRole('button', { name: 'Actions', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Revoke', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Revoke', exact: true }).click();
  await expect(keyRow).toHaveCount(0);
  expect((await request.get(`${api}/v1/me`, { headers: { 'api-key': key } })).status()).toBe(401);
  await page.getByRole('button', { name: 'New', exact: true }).click();
  await expect(page.getByLabel('API token', { exact: true })).toHaveCount(0);
  await page.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
});

test('read-only admin can inspect another account but has no mutation controls', async ({
  page,
  request,
}) => {
  const ownerToken = await token(request, 'itest-webhooks-other');
  const owner = await (
    await request.get(`${api}/v1/me`, { headers: { Authorization: `Bearer ${ownerToken}` } })
  ).json();
  const wallet = owner.wallets[0];
  const created = await request.post(`${api}/v1/me/wallets/${wallet.id}/webhooks`, {
    headers: { Authorization: `Bearer ${ownerToken}` },
    data: { url: `https://127.0.0.1/readonly-${Date.now()}`, event_types: ['invoice.paid'] },
  });
  const webhook = await created.json();
  await signIn(page, request, 'itest-webhooks-read');
  await page.getByRole('button', { name: 'Instance resources', exact: true }).click();
  await expect(page).toHaveURL(/scope=admin/);
  await expect(
    page.getByRole('button', { name: 'Instance resources', exact: true })
  ).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('button', { name: 'Create webhook', exact: true })).toHaveCount(0);
  await page.goto(`/developers?tab=webhooks&scope=admin&id=${webhook.id}`);
  await expect(page.getByText(webhook.url, { exact: true })).toBeVisible();
  for (const action of ['Edit webhook', 'Send test event', 'Rotate secret', 'Delete'])
    await expect(page.getByRole('button', { name: action, exact: true })).toHaveCount(0);
  await page.goto(`/developers?tab=webhooks&id=${webhook.id}&wallet_id=${wallet.id}`);
  await expect(page.getByRole('tabpanel').getByRole('alert')).toContainText(/not found/i);
  await page.goto('/developers?tab=api-keys&scope=admin');
  await expect(page.getByRole('button', { name: 'New', exact: true })).toHaveCount(0);
  await expect(page.getByRole('checkbox')).toHaveCount(0);
  await page.getByRole('button', { name: 'My resources', exact: true }).click();
  await expect(page.getByRole('button', { name: 'New', exact: true })).toBeVisible();
});

test('write-only admin manages known targets without broad read requests', async ({
  page,
  request,
}) => {
  const ownerToken = await token(request, 'itest-webhooks-other');
  const owner = await (
    await request.get(`${api}/v1/me`, { headers: { Authorization: `Bearer ${ownerToken}` } })
  ).json();
  const forbiddenReads: string[] = [];
  page.on('request', (req) => {
    if (
      req.method() === 'GET' &&
      /\/v1\/(webhooks|wallets|accounts|api-keys)(?:[/?]|$)/.test(req.url())
    )
      forbiddenReads.push(req.url());
  });
  await signIn(page, request, 'itest-webhooks-write');
  await page.getByRole('button', { name: 'Instance resources', exact: true }).click();
  await expect(page).toHaveURL(/scope=admin/);
  await expect(
    page.getByRole('button', { name: 'Instance resources', exact: true })
  ).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Create webhook', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Wallet ID', { exact: true }).fill(owner.wallets[0].id);
  await page
    .getByRole('dialog')
    .getByLabel('Destination URL')
    .fill(`https://127.0.0.1/writeonly-${Date.now()}`);
  await page
    .getByRole('dialog')
    .getByRole('checkbox', { name: 'invoice.paid', exact: true })
    .check();
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Create webhook', exact: true })
    .click();
  await page.getByRole('checkbox', { name: 'I have saved this secret securely' }).check();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.getByLabel('Webhook ID', { exact: true })).not.toHaveValue('');
  await expect(page).toHaveURL(/&id=/);
  const knownWebhookId = await page.getByLabel('Webhook ID', { exact: true }).inputValue();
  await page.goBack();
  await expect(page.getByLabel('Webhook ID', { exact: true })).toHaveValue('');
  await page.goForward();
  await expect(page.getByLabel('Webhook ID', { exact: true })).toHaveValue(knownWebhookId);

  await confirmAction(page, 'Disable', 'Disable webhook?');
  await confirmAction(page, 'Enable', 'Enable webhook?');
  await confirmAction(page, 'Delete', 'Delete webhook?');
  await page.getByRole('tab', { name: 'API Keys', exact: true }).click();
  await expect(page.getByRole('tab', { name: 'API Keys', exact: true })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await page.getByRole('button', { name: 'Instance resources', exact: true }).click();
  await expect(page).toHaveURL(/scope=admin/);
  await expect(
    page.getByRole('button', { name: 'Instance resources', exact: true })
  ).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'New', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Token name', { exact: true }).fill(`write-only-${Date.now()}`);
  await dialog.getByLabel('Account ID', { exact: true }).fill(owner.id);
  await expect(dialog.getByRole('checkbox')).toHaveCount(2);
  const createdResponse = page.waitForResponse(
    (response) => response.url().endsWith('/v1/api-keys') && response.request().method() === 'POST'
  );
  await dialog.getByRole('button', { name: 'Generate token', exact: true }).click();
  const createdKey = await (await createdResponse).json();
  await expect(page.getByLabel('API token', { exact: true })).toBeVisible();
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.getByLabel('API key ID', { exact: true }).fill(createdKey.id);
  await page.getByRole('button', { name: 'Revoke', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Revoke', exact: true }).click();
  await expect(page.getByLabel('API key ID', { exact: true })).toHaveValue('');
  expect(
    (await request.get(`${api}/v1/me`, { headers: { 'api-key': createdKey.key } })).status()
  ).toBe(401);
  expect(forbiddenReads).toEqual([]);
});

test('real HTTPS receiver verifies a test and a settled invoice from the dashboard', async ({
  page,
  request,
}) => {
  test.skip(
    !receiver || !receiverFile,
    'Set SWISSKNIFE_E2E_RECEIVER_URL and SWISSKNIFE_E2E_RECEIVER_FILE for the HTTPS acceptance receiver.'
  );
  const { accessToken, account } = await signIn(page, request, 'itest-account');
  const wallet = account.wallets.find(
    (candidate: { asset: { network: string } }) => candidate.asset.network === 'Regtest'
  );
  expect(wallet).toBeTruthy();
  const receiverPath = `/browser-${Date.now()}`;
  const secret = await createWebhook(page, `${receiver}${receiverPath}`, wallet.id);
  await writeFile(`${receiverFile}.secret`, JSON.stringify({ [receiverPath]: secret }), {
    mode: 0o600,
  });
  await confirmAction(page, 'Send test event', 'Send test event?');
  const details = page
    .getByRole('dialog')
    .filter({ has: page.getByRole('heading', { name: 'Delivery details' }) });
  await expect(details.getByText('Delivered', { exact: true })).toBeVisible({ timeout: 30_000 });
  await details.getByRole('button', { name: 'Close', exact: true }).click();
  const invoice = await (
    await request.post(`${api}/v1/me/wallets/${wallet.id}/invoices`, {
      headers: { Authorization: `Bearer ${accessToken}` },
      data: { amount_msat: 1_000_000 },
    })
  ).json();
  // The real regtest counterparty pays through its ordinary CLI, as in backend integration tests.
  const { execFileSync } = await import('node:child_process');
  execFileSync('docker', [
    'exec',
    `${process.env.SWISSKNIFE_ITEST_COMPOSE_PROJECT ?? 'swissknife-itest'}-cln-1`,
    'lightning-cli',
    '--network=regtest',
    'pay',
    invoice.ln_invoice.bolt11,
  ]);
  await expect(page.getByRole('cell', { name: 'invoice.paid' })).toBeVisible({ timeout: 45_000 });
  await expect
    .poll(
      async () => {
        const records = JSON.parse(await readFile(receiverFile!, 'utf8')) as Array<{
          path: string;
          body: string;
          headers: Record<string, string>;
        }>;
        return records
          .filter((row) => row.path === receiverPath)
          .map((row) => {
            const expected = createHmac('sha256', Buffer.from(secret, 'base64url'))
              .update(`${row.headers['x-swissknife-timestamp']}.${row.body}`)
              .digest('hex');
            expect(row.headers['x-swissknife-signature']).toBe(`v1=${expected}`);
            return JSON.parse(row.body).type;
          });
      },
      { timeout: 30_000 }
    )
    .toEqual(expect.arrayContaining(['webhook.test', 'invoice.paid']));
});

test('read/write admin preserves account filters and pagination while editing another account', async ({
  page,
  request,
}) => {
  const ownerToken = await token(request, 'itest-webhooks-other');
  const owner = await (
    await request.get(`${api}/v1/me`, { headers: { Authorization: `Bearer ${ownerToken}` } })
  ).json();
  const wallet = owner.wallets[0];
  const batch = `admin-${Date.now()}`;
  for (let index = 0; index < 11; index += 1) {
    const response = await request.post(`${api}/v1/me/wallets/${wallet.id}/webhooks`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { url: `https://127.0.0.1/${batch}-${index}`, event_types: ['invoice.paid'] },
    });
    expect(response.ok()).toBeTruthy();
  }
  await signIn(page, request, 'itest-full');
  await page.getByRole('button', { name: 'Instance resources', exact: true }).click();
  await expect(page).toHaveURL(/scope=admin/);
  await expect(
    page.getByRole('button', { name: 'Instance resources', exact: true })
  ).toHaveAttribute('aria-pressed', 'true');
  await page.getByLabel('Account ID', { exact: true }).fill(owner.id);
  await page.getByLabel('Wallet ID', { exact: true }).fill(wallet.id);
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`filter_account_id=${owner.id}`));
  await page.getByRole('button', { name: 'Go to next page' }).click();
  await expect(page).toHaveURL(/page=1/);
  await page.getByRole('button', { name: 'Go to previous page' }).click();
  const selected = page.getByRole('link', { name: new RegExp(batch) }).first();
  await selected.click();
  const destinationHeading = page.getByRole('heading', { name: new RegExp(batch) });
  await expect(destinationHeading).toBeVisible();
  const selectedUrl = await destinationHeading.innerText();
  await expect(page.getByText(owner.id, { exact: true })).toBeVisible();
  await confirmAction(page, 'Disable', 'Disable webhook?');
  await page.getByRole('button', { name: 'All webhooks', exact: true }).click();
  await expect(page.getByLabel('Account ID', { exact: true })).toHaveValue(owner.id);
  await page.getByLabel('Subscription status', { exact: true }).click();
  await page.getByRole('option', { name: 'Disabled', exact: true }).click();
  await expect(page.getByRole('link', { name: selectedUrl, exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: new RegExp(batch) })).toHaveCount(1);
  await page.getByRole('button', { name: 'My resources', exact: true }).click();
  await expect(page.getByRole('link', { name: new RegExp(batch) })).toHaveCount(0);
});

test('owner selects another wallet, handles duplicate/deleted resources and recovers failed reads', async ({
  page,
  request,
}) => {
  const ownerToken = await token(request, 'itest-account');
  const headers = { Authorization: `Bearer ${ownerToken}` };
  const owner = await (await request.get(`${api}/v1/me`, { headers })).json();
  const adminToken = await token(request, 'itest-full');
  const adminHeaders = { Authorization: `Bearer ${adminToken}` };
  // Seeded Testnet asset gives this account an independent second wallet.
  const asset = { id: '00000000-0000-4000-8000-000000000002' };
  const response = await request.post(`${api}/v1/wallets`, {
    headers: adminHeaders,
    data: { account_id: owner.id, asset_id: asset.id },
  });
  expect(response.ok()).toBeTruthy();
  const secondWallet = await response.json();
  await signIn(page, request, 'itest-account');
  await page.getByRole('button', { name: 'Create webhook', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Wallet', { exact: true }).click();
  await page.getByRole('option', { name: new RegExp(secondWallet.id) }).click();
  const destination = `https://127.0.0.1/second-wallet-${Date.now()}`;
  await page.getByRole('dialog').getByLabel('Destination URL').fill(destination);
  await page.getByRole('checkbox', { name: 'payment.failed', exact: true }).check();
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Create webhook', exact: true })
    .click();
  await page.getByRole('checkbox', { name: 'I have saved this secret securely' }).check();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`wallet_id=${secondWallet.id}`));
  const detailsUrl = page.url();
  const id = new URL(detailsUrl).searchParams.get('id');
  await page.getByRole('button', { name: 'Create webhook', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Wallet', { exact: true }).click();
  await page.getByRole('option', { name: new RegExp(secondWallet.id) }).click();
  await page.getByRole('dialog').getByLabel('Destination URL').fill(destination);
  await page.getByRole('checkbox', { name: 'payment.failed', exact: true }).check();
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Create webhook', exact: true })
    .click();
  await expect(page.getByText(/already exists/i)).toBeVisible();
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(
    (
      await request.delete(`${api}/v1/me/wallets/${secondWallet.id}/webhooks/${id}`, { headers })
    ).ok()
  ).toBeTruthy();
  await page.reload();
  await expect(page.getByRole('tabpanel').getByRole('alert')).toContainText(/not found/i);
  await expect(page.getByRole('button', { name: 'Edit webhook', exact: true })).toHaveCount(0);
  await page.route('**/v1/me/webhooks?*', (route) =>
    route.fulfill({ status: 503, json: { reason: 'Temporarily unavailable' } })
  );
  await page.getByRole('button', { name: 'All webhooks', exact: true }).click();
  await expect(page.getByRole('tabpanel').getByRole('alert')).toContainText(
    'Temporarily unavailable'
  );
  await page.unroute('**/v1/me/webhooks?*');
  await page.getByRole('button', { name: 'Refresh', exact: true }).click();
  await expect(page.getByRole('tabpanel').getByRole('alert')).toHaveCount(0);
  await page.route('**/v1/me', (route) =>
    route.fulfill({ status: 503, json: { reason: 'Account temporarily unavailable' } })
  );
  await page.reload();
  await expect(page.getByRole('main').getByRole('alert')).toContainText(
    'Account temporarily unavailable'
  );
  await expect(page.getByRole('button', { name: 'Create webhook', exact: true })).toHaveCount(0);
  await page.unroute('**/v1/me');
  await page.getByRole('button', { name: 'Refresh', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'My webhooks', exact: true })).toBeVisible();
});

test('mobile layout and keyboard tabs remain usable; rejected sessions leave the protected page', async ({
  page,
  request,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, request, 'itest-account');
  const webhookTab = page.getByRole('tab', { name: 'Webhooks', exact: true });
  await webhookTab.focus();
  await page.keyboard.press('ArrowUp');
  await expect(page.getByRole('tab', { name: 'API Keys', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('tab', { name: 'API Keys', exact: true })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await expect(webhookTab).toHaveAttribute('aria-selected', 'true');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true
  );
  await page.screenshot({ path: 'test-results/webhooks-mobile.png', fullPage: true });
  await page.route('**/v1/me/webhooks?*', (route) =>
    route.fulfill({ status: 401, json: { reason: 'Credentials revoked' } })
  );
  await page.getByRole('button', { name: 'Refresh', exact: true }).click();
  await expect(page).toHaveURL(/\/login\//);
  expect(await page.evaluate(() => sessionStorage.getItem('jwt_access_token'))).toBeNull();
  await expect(page.getByRole('heading', { name: 'My webhooks', exact: true })).toHaveCount(0);
});

test('administrative API keys retain scope selection, expiration and bulk revocation', async ({
  page,
  request,
}) => {
  const { accessToken, account } = await signIn(page, request, 'itest-full');
  await page.goto('/admin/api-keys');
  await expect(page).toHaveURL(/developers.*tab=api-keys/);
  await page.getByRole('button', { name: 'Instance resources', exact: true }).click();
  await expect(page).toHaveURL(/scope=admin/);
  await expect(
    page.getByRole('button', { name: 'Instance resources', exact: true })
  ).toHaveAttribute('aria-pressed', 'true');
  const prefix = `admin-key-${Date.now()}`;
  const secrets: string[] = [];
  for (let index = 0; index < 2; index += 1) {
    await page.getByRole('button', { name: 'New', exact: true }).click();
    const dialog = page.getByRole('dialog');
    await dialog.getByLabel('Token name', { exact: true }).fill(`${prefix}-${index}`);
    await dialog.getByRole('checkbox', { name: 'read:webhook', exact: true }).check();
    await dialog.getByRole('combobox', { name: 'Account', exact: true }).fill(account.id);
    await expect(page.getByRole('option')).toHaveCount(1);
    await page.getByRole('option').click();
    await dialog.getByRole('button', { name: 'Generate token', exact: true }).click();
    secrets.push(await page.getByLabel('API token', { exact: true }).inputValue());
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    const row = page.getByRole('row').filter({ hasText: `${prefix}-${index}` });
    await row.getByRole('button', { name: 'Scopes', exact: true }).click();
    await expect(page.getByText(/^read:webhook$/i).last()).toBeVisible();
    await row.getByRole('checkbox').check();
  }
  const stored = await (
    await request.get(`${api}/v1/api-keys`, { headers: { Authorization: `Bearer ${accessToken}` } })
  ).json();
  const created = stored.filter((key: { name: string }) => key.name.startsWith(prefix));
  expect(created).toHaveLength(2);
  for (const key of created) {
    expect(key.permissions).toEqual(['read:webhook']);
    expect(key.expires_at).toBeTruthy();
  }
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(page.getByRole('row').filter({ hasText: prefix })).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole('row').filter({ hasText: prefix })).toHaveCount(0);
  for (const key of secrets)
    expect((await request.get(`${api}/v1/me`, { headers: { 'api-key': key } })).status()).toBe(401);
});

test('expired sessions cannot open Developers', async ({ page, request }) => {
  const accessToken = await token(request, 'itest-account');
  await page.clock.install({ time: new Date(Date.now() + 2 * 60 * 60 * 1000) });
  await page.addInitScript(
    (value) => sessionStorage.setItem('jwt_access_token', value),
    accessToken
  );
  await page.goto('/developers?tab=webhooks');
  await expect(page).toHaveURL(/\/login\/.*returnTo=/);
  await expect(page.getByRole('heading', { name: 'My webhooks', exact: true })).toHaveCount(0);
});
