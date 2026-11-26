import { test, expect } from '@playwright/test';

test('uses the viewer’s balance privacy on account cards and payment details', async ({ page }) => {
  const api = 'http://127.0.0.1:1993/v1';
  const created = '2026-01-02T12:00:00Z';
  let hideBalances = false;
  let pendingPreferences: Promise<void> | undefined;
  const wallet = {
    id: 'asset-wallet',
    label: 'Store credit',
    account_id: 'merchant',
    created_at: created,
    asset_id: 'store-credit',
    btc_addresses: [],
    contacts: [],
    invoices: [],
    payments: [],
    asset: { code: 'CRED', display_ticker: 'CRED', protocol: 'taproot_assets', decimals: 2 },
    balance: { available_msat: 1234 },
  };
  await page.route(api + '/system/setup', (route) =>
    route.fulfill({ json: { sign_up_complete: true, welcome_complete: true } })
  );
  await page.route(api + '/me', async (route) => {
    await pendingPreferences;
    return route.fulfill({
      json: {
        id: 'viewer',
        created_at: created,
        wallets: [],
        preferences: { dashboard_settings: { hide_balances: hideBalances } },
      },
    });
  });
  await page.route(api + '/accounts/merchant', (route) =>
    route.fulfill({
      json: {
        id: 'merchant',
        display_name: 'Store',
        created_at: created,
        wallets: [wallet],
        permissions: [],
      },
    })
  );
  await page.route(api + '/payments/receipt', (route) =>
    route.fulfill({
      json: {
        id: 'receipt',
        wallet_id: 'asset-wallet',
        created_at: created,
        ledger: 'Lightning',
        status: 'Settled',
        amount_msat: 97000,
        fee_msat: 781,
      },
    })
  );

  await page.goto('/login');
  await page.getByTestId('mock-oauth2-persona-dev-admin').click();
  await expect
    .poll(() => page.evaluate(() => !!sessionStorage.getItem('jwt_access_token')))
    .toBe(true);

  for (hideBalances of [false, true, false]) {
    const balance = page.getByRole('heading', { name: /CRED$/ });
    // A second account's wallet can arrive before the viewer's privacy preference.
    let releasePreferences: (() => void) | undefined;
    if (hideBalances) {
      pendingPreferences = new Promise((resolve) => {
        releasePreferences = resolve;
      });
    }
    const viewerResponse = page.waitForResponse((response) => response.url() === api + '/me');
    await page.goto('/admin/accounts?id=merchant');
    if (releasePreferences) {
      try {
        await expect(balance).toContainText('••••');
      } finally {
        releasePreferences();
        pendingPreferences = undefined;
      }
    }
    await viewerResponse;
    await expect(balance).toContainText(hideBalances ? '••••' : '12.34');
    await expect(balance).toContainText('CRED');

    await page.goto('/admin/transactions/payments?id=receipt');
    await expect(page.getByText('Fees', { exact: true })).toBeVisible();
    if (hideBalances) {
      await expect(page.getByText(/••••/).first()).toBeVisible();
      await expect(page.getByText(/781 mSats exact/)).toHaveCount(0);
    } else {
      await expect(page.getByText('781 mSats exact')).toBeVisible();
    }
  }
});
