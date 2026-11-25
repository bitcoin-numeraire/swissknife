import type { APIRequestContext } from '@playwright/test';

import { test, expect } from '@playwright/test';

const api = process.env.SWISSKNIFE_POS_REGTEST_API;
const dashboard = process.env.SWISSKNIFE_POS_REGTEST_DASHBOARD;
const rpc = 'http://127.0.0.1:18443/wallet/miner';
const regtestAssetId = '00000000-0000-4000-8000-000000000004';

async function bitcoin(request: APIRequestContext, method: string, params: unknown[]) {
  const response = await request.post(rpc, {
    headers: { Authorization: 'Basic ' + Buffer.from('regtest:regtest').toString('base64') },
    data: { jsonrpc: '2.0', id: 'pos', method, params },
  });
  expect(response.ok()).toBe(true);
  const payload = await response.json();
  expect(payload.error).toBeFalsy();
  return payload.result;
}

test('a merchant can recover a paid regtest sale and immediately start another', async ({
  page,
  request,
}) => {
  test.skip(!api || !dashboard, 'Requires a disposable local SwissKnife instance and dashboard');
  test.setTimeout(90_000);
  for (const url of [api!, dashboard!]) expect(new URL(url).hostname).toBe('127.0.0.1');
  expect((await bitcoin(request, 'getblockchaininfo', [])).chain).toBe('regtest');
  const data = { username: 'pos-owner', password: 'Merchant-Regtest-Only123!' };
  let login = await request.post(api + '/v1/auth/sign-up', { data });
  if (login.status() === 409) login = await request.post(api + '/v1/auth/sign-in', { data });
  expect(login.ok()).toBe(true);
  const { token } = await login.json();
  const headers = { Authorization: 'Bearer ' + token };
  const me = await (await request.get(api + '/v1/me', { headers })).json();
  if (!me.wallets.some((wallet: { asset_id: string }) => wallet.asset_id === regtestAssetId)) {
    const wallet = await request.post(api + '/v1/wallets', {
      headers,
      data: { account_id: me.id, asset_id: regtestAssetId, label: 'Counter' },
    });
    expect(wallet.ok()).toBe(true);
  }
  expect((await request.post(api + '/v1/system/mark-welcome-complete', { headers })).ok()).toBe(
    true
  );
  await page.addInitScript((jwt) => sessionStorage.setItem('jwt_access_token', jwt), token);
  await page.goto(dashboard + '/wallet/pos');
  await page.getByLabel('Amount in sats').fill('25000');
  await page.getByLabel('Description or order reference').fill('Receipt A');
  const address = page.waitForResponse(
    (res) => res.url().endsWith('/bitcoin/addresses') && res.request().method() === 'POST'
  );
  await page.getByRole('button', { name: 'Create payment request' }).click();
  await expect(page.getByText('Bitcoin or Lightning', { exact: true })).toBeVisible();
  const destination = await (await address).json();
  expect(destination.dedicated).toBe(true);
  await bitcoin(request, 'sendtoaddress', [destination.address, 0.00025]);
  if (process.env.SWISSKNIFE_ITEST_PROVIDER?.startsWith('lnd')) {
    await expect(page.getByRole('heading', { name: 'Payment detected' })).toBeVisible({
      timeout: 20000,
    });
  }
  // The tab misses settlement entirely and must recover from durable invoice state.
  await page.goto('about:blank');
  await bitcoin(request, 'generatetoaddress', [
    6,
    await bitcoin(request, 'getnewaddress', ['pos', 'bech32']),
  ]);
  await page.goto(dashboard + '/wallet/pos');
  await expect(page.getByRole('heading', { name: 'Payment received' })).toBeVisible({
    timeout: 30000,
  });
  const detail = await page
    .getByRole('link', { name: 'Open payment details' })
    .getAttribute('href');
  await page.getByRole('button', { name: 'Next sale' }).click();
  await expect(page.getByRole('link', { name: 'Last completed sale' })).toHaveAttribute(
    'href',
    detail!
  );
  await page.getByLabel('Amount in sats').fill('2000');
  await page.getByRole('button', { name: 'Create payment request' }).click();
  await expect(page.getByRole('heading', { name: 'Waiting for payment' })).toBeVisible();
  await page.getByRole('button', { name: 'Stop checkout' }).click();
  await page.reload();
  await expect(page.getByLabel('Amount in sats')).toBeEmpty();
});
