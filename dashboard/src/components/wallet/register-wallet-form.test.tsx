import { vi, it, expect, describe, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

import { RegisterWalletForm } from './register-wallet-form';

const accountId = '00000000-0000-4000-8000-000000000099';
const mainnet = {
  id: '00000000-0000-4000-8000-000000000001',
  code: 'BTC',
  name: 'Bitcoin',
  display_ticker: 'BTC',
  protocol: 'bitcoin',
  network: 'Bitcoin',
};
const signet = {
  ...mainnet,
  id: '00000000-0000-4000-8000-000000000006',
  display_ticker: 'sBTC',
  network: 'Signet',
};
const api = vi.hoisted(() => ({
  catalog: vi.fn(),
  account: vi.fn(),
  registerWallet: vi.fn(),
}));

vi.mock('src/actions/asset', () => ({ useListAssets: api.catalog }));
vi.mock('src/actions/account', () => ({ useGetAccount: api.account }));
vi.mock('src/lib/swissknife', () => ({ registerWallet: api.registerWallet }));
vi.mock('src/locales', () => ({ useTranslate: () => ({ t: (key: string) => key }) }));
vi.mock('src/components/snackbar', () => ({ toast: { success: vi.fn() } }));

beforeEach(() => {
  vi.resetAllMocks();
  api.catalog.mockReturnValue({ data: [mainnet, signet] });
  api.account.mockReturnValue({ account: { id: accountId, wallets: [{ asset_id: mainnet.id }] } });
});

describe('wallet registration', () => {
  it('offers only unowned assets and submits the selected network', async () => {
    const onSuccess = vi.fn();
    render(<RegisterWalletForm accountId={accountId} onSuccess={onSuccess} />);
    expect(screen.getByRole('button', { name: 'register' })).toBeDisabled();
    fireEvent.mouseDown(screen.getByRole('combobox'));
    const options = await screen.findAllByRole('option');
    expect(options).toHaveLength(1);
    expect(options[0]).toHaveTextContent('Signet');
    fireEvent.click(options[0]);
    fireEvent.click(screen.getByRole('button', { name: 'register' }));
    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(api.registerWallet).toHaveBeenCalledWith({
      body: { account_id: accountId, asset_id: signet.id },
    });
  });

  it('keeps submission disabled when there are no available assets', () => {
    api.catalog.mockReturnValue({ data: [mainnet] });
    render(<RegisterWalletForm accountId={accountId} onSuccess={vi.fn()} />);
    expect(screen.getByText('register_wallet.no_assets')).toBeVisible();
    expect(screen.getByRole('button', { name: 'register' })).toBeDisabled();
  });

  it('waits for account ownership and reports a catalog error', () => {
    api.account.mockReturnValue({ accountLoading: true });
    const view = render(<RegisterWalletForm accountId={accountId} onSuccess={vi.fn()} />);
    expect(screen.getByText('register_wallet.loading_assets')).toBeVisible();
    expect(screen.getByRole('combobox')).toBeDisabled();
    api.catalog.mockReturnValue({ error: new Error('unavailable') });
    view.rerender(<RegisterWalletForm accountId={accountId} onSuccess={vi.fn()} />);
    expect(screen.getByRole('alert')).toHaveTextContent('register_wallet.asset_load_error');
    expect(screen.getByRole('button', { name: 'register' })).toBeDisabled();
  });
});
