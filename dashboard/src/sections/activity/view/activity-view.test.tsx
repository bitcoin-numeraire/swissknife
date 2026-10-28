import type { Account, Invoice, Payment } from 'src/lib/swissknife';

import { vi, it, expect, describe, beforeEach } from 'vitest';
import { render, screen, within, waitFor, fireEvent } from '@testing-library/react';

import { ThemeProvider } from '@mui/material/styles';
import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs';
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider';

import { Permission } from 'src/lib/swissknife';
import { createTheme } from 'src/theme/create-theme';
import { client } from 'src/lib/swissknife/client.gen';

import { ActivityView, AdminTransactionsView } from './activity-view';

const mocks = vi.hoisted(() => ({
  permissions: [] as string[],
  accounts: [] as Account[],
  invoices: [] as Invoice[],
  payments: [] as Payment[],
  accountError: undefined as Error | undefined,
  listAccounts: vi.fn(),
  mutate: vi.fn(),
  success: vi.fn(),
  handleActionError: vi.fn(),
  params: new URLSearchParams(),
}));

vi.mock('src/routes/hooks', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useSearchParams: () => mocks.params,
}));
vi.mock('src/locales', () => ({
  useTranslate: () => ({
    t: (key: string, options?: { count?: number }) =>
      options?.count === undefined ? key : `${key}:${options.count}`,
  }),
  formatNumberLocale: () => 'en-US',
}));
vi.mock('src/auth/hooks', () => ({
  useAuthContext: () => ({ user: { permissions: mocks.permissions } }),
}));
vi.mock('src/components/settings', () => ({
  useSettingsContext: () => ({ state: { currency: 'USD', displayUnit: 'sats' } }),
}));
vi.mock('src/components/chart', () => ({ Chart: () => null, useChart: () => ({}) }));
vi.mock('src/sections/wallet/money-drawers', () => ({
  SendMoneyDrawer: () => null,
  ReceiveMoneyDrawer: () => null,
}));
vi.mock('src/sections/transaction/transaction-quick-drawer', () => ({
  TransactionQuickDrawer: ({ open }: { open: boolean }) =>
    open ? <div>Transaction drawer</div> : null,
}));
vi.mock('src/actions/account', () => ({
  useListAccounts: (enabled: boolean) => {
    mocks.listAccounts(enabled);
    return { accounts: enabled ? mocks.accounts : undefined, accountsError: mocks.accountError };
  },
}));
vi.mock('src/actions/invoices', () => ({
  useListInvoices: () => ({ invoices: mocks.invoices, invoicesLoading: false }),
}));
vi.mock('src/actions/payments', () => ({
  useListPayments: () => ({ payments: mocks.payments, paymentsLoading: false }),
}));
vi.mock('src/actions/account-wallet', () => ({
  useActiveWallet: () => ({
    wallet: {
      id: 'personal-wallet',
      balance: { available_msat: 0 },
      invoices: mocks.invoices,
      payments: mocks.payments,
    },
  }),
}));
vi.mock('src/actions/mempool-space', () => ({
  useFetchFiatPrices: () => ({ fiatPrices: { USD: 0, CHF: 0, EUR: 0 } }),
}));
vi.mock('swr', async (importOriginal) => ({
  ...(await importOriginal<object>()),
  mutate: mocks.mutate,
}));
vi.mock('src/components/snackbar', () => ({ toast: { success: mocks.success, info: vi.fn() } }));
vi.mock('src/utils/errors', async (importOriginal) => ({
  ...(await importOriginal<object>()),
  handleActionError: mocks.handleActionError,
}));

const fetcher = vi.fn<typeof fetch>();
const common = {
  created_at: new Date('2026-10-28T12:00:00Z'),
  ledger: 'Lightning',
  amount_msat: 1000,
};
const invoice = {
  ...common,
  id: 'shared-id',
  wallet_id: 'wallet-treasury',
  description: 'Selected invoice',
  status: 'Expired',
} as Invoice;
const payment = {
  ...common,
  id: 'shared-id',
  wallet_id: 'wallet-shop',
  description: 'Selected payment',
  status: 'Failed',
} as Payment;

function response(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}
function requests() {
  return fetcher.mock.calls.map(([input]) => {
    const request = input as Request;
    const url = new URL(request.url);
    expect(url.origin).toBe('https://transactions.test');
    return {
      method: request.method,
      path: url.pathname,
      query: {
        ...Object.fromEntries(url.searchParams),
        ids: url.searchParams.getAll('ids').join(','),
      },
    };
  });
}
function renderView(wallet = false) {
  return render(
    <ThemeProvider theme={createTheme()}>
      <LocalizationProvider dateAdapter={AdapterDayjs}>
        {wallet ? <ActivityView /> : <AdminTransactionsView />}
      </LocalizationProvider>
    </ThemeProvider>
  );
}
function selectRow(description: string) {
  fireEvent.click(
    within(screen.getByRole('row', { name: new RegExp(description) })).getByRole('checkbox')
  );
}
async function confirmDelete() {
  fireEvent.click(screen.getByRole('button', { name: 'activity_view.delete_selected' }));
  const dialog = screen.getByRole('dialog');
  expect(fetcher).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole('button', { name: /^delete$/ }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
}

beforeEach(() => {
  vi.clearAllMocks();
  fetcher.mockReset();
  fetcher.mockImplementation(async () => response(1));
  client.setConfig({ baseUrl: 'https://transactions.test', fetch: fetcher, throwOnError: false });
  mocks.permissions = [
    Permission.READ_TRANSACTION,
    Permission.WRITE_TRANSACTION,
    Permission.READ_ACCOUNT,
  ];
  mocks.invoices = [
    invoice,
    { ...invoice, id: 'other-invoice', description: 'Unselected invoice' },
  ];
  mocks.payments = [
    payment,
    { ...payment, id: 'other-payment', description: 'Unselected payment' },
  ];
  mocks.accounts = [
    {
      id: 'account-treasury',
      display_name: 'Treasury',
      wallets: [
        {
          id: 'wallet-treasury',
          label: 'Cold reserve',
          asset: { name: 'Bitcoin', display_ticker: 'BTC', network: 'mainnet' },
        },
      ],
    },
    {
      id: 'account-shop',
      identity: { subject: 'auth0|shop-owner' },
      wallets: [{ id: 'wallet-shop', label: 'Checkout' }],
    },
  ] as Account[];
  mocks.accountError = undefined;
  mocks.params = new URLSearchParams();
});

describe('administrative transactions', () => {
  it('shows linked account and wallet labels and keeps owner links separate from the transaction drawer', () => {
    renderView();
    const row = within(screen.getByRole('row', { name: /Selected invoice/ }));
    expect(row.getByRole('link', { name: 'Treasury' })).toHaveAttribute(
      'href',
      '/admin/accounts?id=account-treasury'
    );
    expect(row.getByRole('link', { name: 'Cold reserve' })).toHaveAttribute(
      'href',
      '/admin/wallets?id=wallet-treasury'
    );
    expect(row.getByText('BTC · mainnet · wallet-treasury')).toBeVisible();
    fireEvent.click(row.getByRole('link', { name: 'Treasury' }));
    expect(screen.queryByText('Transaction drawer')).not.toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'auth0|shop-owner' })).toHaveLength(2);
  });

  it('deletes exactly the selected mixed resource IDs after confirmation', async () => {
    renderView();
    selectRow('Selected invoice');
    selectRow('Selected payment');
    expect(screen.getByText('2 selected')).toBeVisible();
    await confirmDelete();
    expect(requests()).toEqual([
      { method: 'DELETE', path: '/v1/invoices', query: { ids: 'shared-id' } },
      { method: 'DELETE', path: '/v1/payments', query: { ids: 'shared-id' } },
    ]);
    expect(mocks.success).toHaveBeenCalledWith('transaction_list.transactions_deleted:2');
    expect(mocks.mutate).toHaveBeenCalledTimes(2);
    expect(screen.queryByText('2 selected')).not.toBeInTheDocument();
  });

  it('searches ownership and removes hidden rows from the selection before select-all deletion', async () => {
    renderView();
    selectRow('Selected payment');
    fireEvent.change(screen.getByPlaceholderText('transaction_table_toolbar.search_placeholder'), {
      target: { value: 'Cold reserve' },
    });
    expect(screen.queryByRole('row', { name: /Selected payment/ })).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'activity_view.delete_selected' })
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('checkbox', { name: 'All row Checkbox' }));
    await confirmDelete();
    expect(requests()).toEqual([
      { method: 'DELETE', path: '/v1/invoices', query: { ids: 'shared-id,other-invoice' } },
    ]);
  });

  it('selects filtered transactions across pages and adjusts the page after refresh', async () => {
    mocks.invoices = Array.from({ length: 30 }, (_, i) => ({
      ...invoice,
      id: `invoice-${i}`,
      description: `Batch invoice ${i}`,
    }));
    mocks.payments = [];
    const view = renderView();
    fireEvent.click(screen.getByRole('button', { name: 'Go to next page' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'All row Checkbox' }));
    expect(screen.getByText('30 selected')).toBeVisible();
    await confirmDelete();
    expect(requests()[0].query.ids.split(',')).toHaveLength(30);
    expect(fetcher).toHaveBeenCalledOnce();
    mocks.invoices = [invoice];
    view.rerender(
      <ThemeProvider theme={createTheme()}>
        <LocalizationProvider dateAdapter={AdapterDayjs}>
          <AdminTransactionsView />
        </LocalizationProvider>
      </ThemeProvider>
    );
    expect(screen.getByRole('row', { name: /Selected invoice/ })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Go to previous page' })).toBeDisabled();
  });

  it('cancels confirmation without deleting or clearing the selection', async () => {
    renderView();
    selectRow('Selected invoice');
    fireEvent.click(screen.getByRole('button', { name: 'activity_view.delete_selected' }));
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(fetcher).not.toHaveBeenCalled();
    expect(screen.getByText('1 selected')).toBeVisible();
  });

  it('keeps failed payments selected and refreshes invoices when only the invoice request succeeds', async () => {
    const error = { reason: 'Cannot delete pending payments.' };
    fetcher.mockResolvedValueOnce(response(1)).mockResolvedValueOnce(response(error, 422));
    renderView();
    selectRow('Selected invoice');
    selectRow('Selected payment');
    await confirmDelete();
    expect(mocks.handleActionError).toHaveBeenCalledWith(error);
    expect(mocks.success).toHaveBeenCalledWith('transaction_list.transactions_deleted:1');
    expect(mocks.mutate).toHaveBeenCalledTimes(2);
    expect(
      within(screen.getByRole('row', { name: /Selected payment/ })).getByRole('checkbox')
    ).toBeChecked();
    expect(
      within(screen.getByRole('row', { name: /Selected invoice/ })).getByRole('checkbox')
    ).not.toBeChecked();
  });

  it('does not report success or clear selection when the API rejects deletion', async () => {
    fetcher.mockImplementation(async () => response({ reason: 'Forbidden' }, 403));
    renderView();
    selectRow('Selected payment');
    await confirmDelete();
    expect(requests()).toEqual([
      { method: 'DELETE', path: '/v1/payments', query: { ids: 'shared-id' } },
    ]);
    expect(mocks.success).not.toHaveBeenCalled();
    expect(mocks.mutate).not.toHaveBeenCalled();
    expect(screen.getByText('1 selected')).toBeVisible();
  });

  it('does not offer deletion or request accounts without the corresponding permissions', () => {
    mocks.permissions = [Permission.READ_TRANSACTION];
    renderView();
    selectRow('Selected invoice');
    expect(
      screen.queryByRole('button', { name: 'activity_view.delete_selected' })
    ).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'clean' })).toBeDisabled();
    expect(mocks.listAccounts).toHaveBeenCalledWith(false);
    expect(screen.getAllByRole('link', { name: 'wallet-treasury' })).toHaveLength(2);
  });

  it('keeps transactions accessible if ownership metadata cannot be loaded', () => {
    mocks.accounts = [];
    mocks.accountError = new Error('Unavailable');
    renderView();
    expect(screen.getByRole('alert')).toHaveTextContent('activity_view.ownership_unavailable');
    expect(screen.getByRole('row', { name: /Selected invoice/ })).toBeVisible();
    expect(screen.getAllByRole('link', { name: 'wallet-treasury' })).toHaveLength(2);
  });

  it('does not expose administrative ownership or bulk deletion on the wallet activity screen', () => {
    renderView(true);
    selectRow('Selected invoice');
    expect(
      screen.queryByRole('button', { name: 'activity_view.delete_selected' })
    ).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Treasury' })).not.toBeInTheDocument();
    expect(mocks.listAccounts).not.toHaveBeenCalled();
  });
});
