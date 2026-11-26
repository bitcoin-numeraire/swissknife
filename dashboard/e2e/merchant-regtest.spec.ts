import type { Page, APIRequestContext } from '@playwright/test';

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

async function openMerchant(page: Page, request: APIRequestContext) {
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
  return headers;
}

test('a merchant can recover a partial regtest sale, collect the balance and start another', async ({
  page,
  request,
}) => {
  const headers = await openMerchant(page, request);
  await page.getByLabel('Amount in sats').fill('25000');
  await page.getByLabel('Description or order reference').fill('Receipt A');
  const address = page.waitForResponse(
    (res) => res.url().endsWith('/bitcoin/addresses') && res.request().method() === 'POST'
  );
  await page.getByRole('button', { name: 'Create payment request' }).click();
  await expect(page.getByText('Bitcoin or Lightning', { exact: true })).toBeVisible();
  const destination = await (await address).json();
  expect(destination.dedicated).toBe(true);
  expect(destination.description).toBe('Receipt A');
  await bitcoin(request, 'sendtoaddress', [destination.address, 0.0001]);
  if (process.env.SWISSKNIFE_ITEST_PROVIDER?.startsWith('lnd')) {
    await expect(page.getByRole('heading', { name: 'Payment detected' })).toBeVisible({
      timeout: 20000,
    });
  }
  // Credit the first transfer while the checkout is suspended.
  await page.goto('about:blank');
  const miningAddress = await bitcoin(request, 'getnewaddress', ['pos', 'bech32']);
  await bitcoin(request, 'generatetoaddress', [6, miningAddress]);
  const history = (url: URL) =>
    url.pathname.endsWith('/invoices') &&
    url.searchParams.get('bitcoin_address') === destination.address;
  let release!: () => void;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route(history, async (route) => {
    await held;
    await route.continue();
  });
  const requested = page.waitForRequest((req) => history(new URL(req.url())));
  try {
    await page.goto(dashboard + '/wallet/pos', { waitUntil: 'domcontentloaded' });
    await requested;
    await expect(page.getByRole('heading', { name: 'Waiting for payment' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Copy', exact: true })).toHaveCount(0);
    await expect(page.locator('canvas')).toHaveCount(0);
  } finally {
    release();
    await page.unrouteAll({ behavior: 'wait' });
  }
  await expect(page.getByRole('heading', { name: 'Partial payment' })).toBeVisible({
    timeout: 30000,
  });
  await expect(page.getByText('On-chain Bitcoin · 15,000 sats')).toBeVisible();
  await page
    .context()
    .grantPermissions(['clipboard-read', 'clipboard-write'], { origin: dashboard! });
  await page.getByRole('button', { name: 'Copy', exact: true }).click();
  const remaining = await page.evaluate(() => navigator.clipboard.readText());
  expect(remaining).toBe(`bitcoin:${destination.address}?amount=0.00015`);
  const payment = new URL(remaining);
  await bitcoin(request, 'sendtoaddress', [
    payment.pathname,
    Number(payment.searchParams.get('amount')),
  ]);
  await bitcoin(request, 'generatetoaddress', [6, miningAddress]);
  await expect(page.getByRole('heading', { name: 'Payment received' })).toBeVisible({
    timeout: 30000,
  });
  await expect(page.getByRole('button', { name: 'Copy', exact: true })).toHaveCount(0);
  const receipts = page.getByRole('link', { name: 'Open payment details' });
  await expect(receipts).toHaveCount(2);
  const links = (
    await receipts.evaluateAll((items) => items.map((item) => item.getAttribute('href')))
  ).sort();
  await page.getByRole('button', { name: 'Next sale' }).click();
  const previous = page.getByRole('link', { name: 'Last completed sale' });
  await expect(previous).toHaveCount(2);
  await page.reload();
  await expect(previous).toHaveCount(2);
  expect(
    (await previous.evaluateAll((items) => items.map((item) => item.getAttribute('href')))).sort()
  ).toEqual(links);
  const historyUrl = new URL(api + '/v1/me/wallets/' + destination.wallet_id + '/invoices');
  historyUrl.searchParams.set('bitcoin_address', destination.address);
  const historyResponse = await request.get(historyUrl.href, { headers });
  expect(historyResponse.ok()).toBe(true);
  const entries = await historyResponse.json();
  expect(entries.map((entry: { description: string }) => entry.description)).toEqual([
    'Receipt A',
    'Receipt A',
  ]);
  for (const entry of entries) {
    const link = previous.and(page.locator(`a[href="/wallet/invoices?id=${entry.id}"]`));
    await link.click();
    await expect(
      page.getByRole('heading', { name: 'Receipt A', exact: true, level: 4 })
    ).toBeVisible();
    await page.goto(dashboard + '/wallet/pos');
  }
  await page.getByLabel('Amount in sats').fill('2000');
  await page.getByRole('button', { name: 'Create payment request' }).click();
  await expect(page.getByRole('heading', { name: 'Waiting for payment' })).toBeVisible();
  await page.getByRole('button', { name: 'Stop checkout' }).click();
  await page.reload();
  await expect(page.getByLabel('Amount in sats')).toBeEmpty();
  await expect(previous).toHaveCount(2);
});

test('a cashier can collect a sale partly from another SwissKnife wallet', async ({
  page,
  request,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'vibrate', {
      value: () => {
        sessionStorage.setItem(
          'test-vibrations',
          String(Number(sessionStorage.getItem('test-vibrations')) + 1)
        );
        return true;
      },
    });
  });
  const headers = await openMerchant(page, request);
  await page.getByLabel('Vibration').check();
  const account = await request.post(api + '/v1/accounts', {
    headers,
    data: { display_name: 'Checkout customer', permissions: [] },
  });
  expect(account.ok()).toBe(true);
  const wallet = await request.post(api + '/v1/wallets', {
    headers,
    data: { account_id: (await account.json()).id, asset_id: regtestAssetId },
  });
  expect(wallet.ok()).toBe(true);
  const payer = (await wallet.json()).id;
  const address = await request.post(api + '/v1/bitcoin/addresses', {
    headers,
    data: { wallet_id: payer },
  });
  expect(address.ok()).toBe(true);
  await bitcoin(request, 'sendtoaddress', [(await address.json()).address, 0.0005]);
  const miner = await bitcoin(request, 'getnewaddress', ['pos', 'bech32']);
  await bitcoin(request, 'generatetoaddress', [6, miner]);
  await expect
    .poll(async () => {
      const funded = await request.get(api + '/v1/wallets/' + payer, { headers });
      return (await funded.json()).balance.available_msat;
    })
    .toBe(50_000_000);

  await page.getByLabel('Amount in sats').fill('25000');
  await page.getByLabel('Description or order reference').fill('Order 47');
  const issued = page.waitForResponse(
    (res) => res.url().endsWith('/bitcoin/addresses') && res.request().method() === 'POST'
  );
  await page.getByRole('button', { name: 'Create payment request' }).click();
  const destination = (await (await issued).json()).address;
  const paid = await request.post(api + '/v1/payments', {
    headers,
    data: {
      wallet_id: payer,
      input: destination,
      amount_msat: 12_000_000,
      comment: 'Customer note',
    },
  });
  expect(paid.ok()).toBe(true);
  expect(await paid.json()).toMatchObject({ ledger: 'Internal', description: 'Customer note' });
  await expect(page.getByRole('heading', { name: 'Partial payment' })).toBeVisible({
    timeout: 30000,
  });
  await page.reload();
  await expect(page.getByText('On-chain Bitcoin · 13,000 sats')).toBeVisible();
  await bitcoin(request, 'sendtoaddress', [destination, 0.00013]);
  await bitcoin(request, 'generatetoaddress', [6, miner]);
  await expect(page.getByRole('heading', { name: 'Payment received' })).toBeVisible({
    timeout: 30000,
  });
  const receipts = page.getByRole('link', { name: 'Open payment details' });
  await expect(receipts).toHaveCount(2);
  const vibrations = () => page.evaluate(() => sessionStorage.getItem('test-vibrations'));
  await expect.poll(vibrations).toBe('1');

  // Another receipt can settle after the cashier has already seen success.
  await bitcoin(request, 'sendtoaddress', [destination, 0.00001]);
  await bitcoin(request, 'generatetoaddress', [6, miner]);
  await expect(receipts).toHaveCount(3, { timeout: 30000 });
  await page.getByRole('button', { name: 'Next sale' }).click();
  await page.reload();
  await expect(page.getByRole('link', { name: 'Last completed sale' })).toHaveCount(3);
  expect(await vibrations()).toBe('1');
});
