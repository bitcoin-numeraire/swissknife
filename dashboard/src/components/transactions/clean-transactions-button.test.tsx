import { vi, it, expect, describe, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

import { client } from 'src/lib/swissknife/client.gen';

import { TransactionType } from 'src/types/transaction';

import { CleanTransactionsButton } from './clean-transactions-button';

const mocks = vi.hoisted(() => ({
  activeWallet: { id: 'operator-wallet' } as { id: string } | undefined,
  success: vi.fn(),
  info: vi.fn(),
  handleActionError: vi.fn(),
}));

vi.mock('src/actions/account-wallet', () => ({
  useActiveWallet: () => ({ wallet: mocks.activeWallet }),
}));
vi.mock('src/locales', () => ({
  useTranslate: () => ({
    t: (key: string, options?: { count: number }) => (options ? `${key}:${options.count}` : key),
  }),
}));
vi.mock('src/components/snackbar', () => ({
  toast: { success: mocks.success, info: mocks.info },
}));
vi.mock('src/utils/errors', () => ({ handleActionError: mocks.handleActionError }));

const fetcher = vi.fn<typeof fetch>();
const onSuccess = vi.fn();

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
    expect(url.origin).toBe('https://cleanup.test');
    return {
      method: request.method,
      path: url.pathname,
      query: Object.fromEntries(url.searchParams),
    };
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.activeWallet = { id: 'operator-wallet' };
  fetcher.mockReset();
  fetcher.mockResolvedValue(response(0));
  client.setConfig({ baseUrl: 'https://cleanup.test', fetch: fetcher, throwOnError: false });
});

describe('CleanTransactionsButton', () => {
  it('cleans administrative transactions across wallets rather than the operator wallet', async () => {
    fetcher.mockImplementation(async (input) => {
      const path = new URL((input as Request).url).pathname;
      return response(path === '/v1/invoices' ? 2 : path === '/v1/payments' ? 3 : 0);
    });
    render(
      <CleanTransactionsButton scope="admin" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(requests()).toEqual([
      { method: 'DELETE', path: '/v1/invoices', query: { status: 'Expired' } },
      { method: 'DELETE', path: '/v1/payments', query: { status: 'Failed' } },
    ]);
    expect(mocks.success).toHaveBeenCalledWith(
      'clean_transactions_button.invoices_deleted_success:2'
    );
    expect(mocks.success).toHaveBeenCalledWith(
      'clean_transactions_button.payments_deleted_success:3'
    );
  });

  it('does not require a personal wallet for administrative cleanup', async () => {
    mocks.activeWallet = undefined;
    fetcher.mockImplementation(async () => response(1));
    render(
      <CleanTransactionsButton scope="admin" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    expect(screen.getByRole('button', { name: 'Clean' })).toBeEnabled();
    fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(requests().map((request) => request.path)).toEqual(['/v1/invoices', '/v1/payments']);
  });

  it.each([
    { transactionType: TransactionType.INVOICE, path: '/v1/invoices', status: 'Expired' },
    { transactionType: TransactionType.PAYMENT, path: '/v1/payments', status: 'Failed' },
  ])(
    'cleans only the selected $transactionType kind',
    async ({ transactionType, path, status }) => {
      fetcher.mockResolvedValue(response(2));
      render(
        <CleanTransactionsButton
          scope="admin"
          transactionType={transactionType}
          onSuccess={onSuccess}
        >
          Clean
        </CleanTransactionsButton>
      );

      fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

      await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
      expect(requests()).toEqual([{ method: 'DELETE', path, query: { status } }]);
    }
  );

  it('keeps wallet cleanup scoped to the wallet supplied by the page', async () => {
    fetcher.mockImplementation(async () => response(1));
    render(
      <CleanTransactionsButton scope="wallet" walletId="selected-wallet" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(requests()).toEqual([
      { method: 'DELETE', path: '/v1/me/wallets/selected-wallet/invoices', query: {} },
      { method: 'DELETE', path: '/v1/me/wallets/selected-wallet/payments', query: {} },
    ]);
  });

  it('disables wallet cleanup until a wallet is selected', () => {
    render(
      <CleanTransactionsButton scope="wallet" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    expect(screen.getByRole('button', { name: 'Clean' })).toBeDisabled();
    expect(fetcher).not.toHaveBeenCalled();
  });

  it('reports an empty cleanup result and refreshes stale transaction lists', async () => {
    fetcher.mockImplementation(async () => response(0));
    render(
      <CleanTransactionsButton scope="admin" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

    await waitFor(() =>
      expect(mocks.info).toHaveBeenCalledWith('clean_transactions_button.nothing_to_clean')
    );
    expect(onSuccess).toHaveBeenCalledOnce();
    expect(mocks.success).not.toHaveBeenCalled();
  });

  it('surfaces API permission errors rather than treating them as empty results', async () => {
    const error = { status: '403 Forbidden', reason: 'Missing required permission.' };
    fetcher.mockImplementation(async () => response(error, 403));
    render(
      <CleanTransactionsButton scope="admin" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

    await waitFor(() => expect(mocks.handleActionError).toHaveBeenCalledWith(error));
    expect(fetcher).toHaveBeenCalledOnce();
    expect(onSuccess).not.toHaveBeenCalled();
    expect(mocks.info).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Clean' })).toBeEnabled();
  });

  it('refreshes successful deletions even if the second cleanup request fails', async () => {
    const error = { status: '500 Internal Server Error', reason: 'Temporary database failure.' };
    fetcher.mockResolvedValueOnce(response(2)).mockResolvedValueOnce(response(error, 500));
    render(
      <CleanTransactionsButton scope="admin" onSuccess={onSuccess}>
        Clean
      </CleanTransactionsButton>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Clean' }));

    await waitFor(() => expect(mocks.handleActionError).toHaveBeenCalledWith(error));
    expect(mocks.success).toHaveBeenCalledWith(
      'clean_transactions_button.invoices_deleted_success:2'
    );
    expect(onSuccess).toHaveBeenCalledOnce();
    expect(mocks.info).not.toHaveBeenCalled();
  });
});
