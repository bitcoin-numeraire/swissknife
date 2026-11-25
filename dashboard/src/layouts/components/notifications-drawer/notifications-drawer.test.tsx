import { SWRConfig } from 'swr';
import { it, vi, expect, describe } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

import { ThemeProvider } from '@mui/material/styles';

import { createTheme } from 'src/theme/create-theme';
import {
  listAccountNotifications,
  updateAccountNotification,
  markAccountNotificationsRead,
} from 'src/lib/swissknife';

import { NotificationsDrawer } from './index';

vi.mock('src/contexts/account', () => ({
  useAccountContext: () => ({
    account: { id: 'alice' },
    wallets: [{ id: 'wallet-2', label: 'Shop' }],
  }),
}));
vi.mock('src/locales', () => ({
  useTranslate: () => ({
    currentLang: { numberFormat: { code: 'en-US' } },
    t: (key: string, values?: { count?: number; amount?: string }) =>
      [key, values?.count ?? values?.amount].filter((value) => value != null).join(' '),
  }),
}));
vi.mock('src/components/settings', () => ({
  useSettingsContext: () => ({ state: { hideBalances: false } }),
}));
vi.mock(import('src/lib/swissknife'), async (original) => ({
  ...(await original()),
  listAccountNotifications: vi.fn(),
  updateAccountNotification: vi.fn(),
  markAccountNotificationsRead: vi.fn(),
}));

describe('notification drawer', () => {
  it('refreshes the badge after a read and archive, and uses the observed cursor for mark-all', async () => {
    const items = [4, 3, 2].map((id) => ({
      source_event_id: id,
      resource_id: 'tx-' + id,
      event_type: 'invoice.paid' as const,
      wallet_id: 'wallet-2',
      ledger: 'Lightning' as const,
      created_at: new Date(),
      amount_msat: '9007199254740993',
      description: 'Receipt ' + id,
      is_read: false,
      archived: false,
    }));
    vi.mocked(listAccountNotifications).mockImplementation(async (options) => ({
      data: {
        items: items.filter((item) => item.archived === Boolean(options?.query?.archived)),
        unread_count: items.filter((item) => !item.is_read).length,
        next_before: null,
      },
      request: new Request('https://example.test'),
      response: new Response(),
    }));
    vi.mocked(updateAccountNotification).mockImplementation(async (options) => {
      const item = items.find((row) => row.source_event_id === options.path.id)!;
      item.is_read = true;
      if (options.body.action === 'archive') item.archived = true;
      return {
        data: undefined,
        request: new Request('https://example.test'),
        response: new Response(null, { status: 204 }),
      };
    });
    vi.mocked(markAccountNotificationsRead).mockImplementation(async (options) => {
      items
        .filter((item) => item.source_event_id <= options.body.through)
        .forEach((item) => {
          item.is_read = true;
        });
      return {
        data: undefined,
        request: new Request('https://example.test'),
        response: new Response(null, { status: 204 }),
      };
    });
    render(
      <SWRConfig value={{ provider: () => new Map(), dedupingInterval: 0 }}>
        <ThemeProvider theme={createTheme()}>
          <NotificationsDrawer />
        </ThemeProvider>
      </SWRConfig>
    );
    fireEvent.click(await screen.findByRole('button', { name: 'notification_inbox.open 3' }));
    expect(
      await screen.findAllByText('notification_inbox.amount 9,007,199,254,740,993')
    ).toHaveLength(3);
    expect(screen.getAllByRole('link', { name: 'notification_inbox.view' })[0]).toHaveAttribute(
      'href',
      '/wallet/invoices?id=tx-4&wallet_id=wallet-2'
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'notification_inbox.read' })[0]);
    await waitFor(() =>
      expect(screen.getByLabelText('notification_inbox.open 2')).toBeInTheDocument()
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'notification_inbox.archive' })[0]);
    await waitFor(() => expect(screen.queryByText('Receipt 4')).not.toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'notification_inbox.all_read' }));
    await waitFor(() =>
      expect(markAccountNotificationsRead).toHaveBeenCalledWith({ body: { through: 3 } })
    );
    await waitFor(() =>
      expect(screen.getByLabelText('notification_inbox.open 0')).toBeInTheDocument()
    );
    fireEvent.click(screen.getByRole('tab', { name: 'notification_inbox.archived' }));
    expect(await screen.findByText('Receipt 4')).toBeInTheDocument();
  });
});
