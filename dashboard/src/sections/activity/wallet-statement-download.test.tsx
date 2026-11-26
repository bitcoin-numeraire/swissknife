import dayjs from 'dayjs';
import { it, vi, expect, afterEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs';
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider';

import { client } from 'src/lib/swissknife/client.gen';

import { statementPeriod, WalletStatementDownload } from './wallet-statement-download';

vi.mock('src/locales', () => ({ useTranslate: () => ({ t: (key: string) => key }) }));

const config = client.getConfig();
afterEach(() => {
  client.setConfig(config);
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('exports every selected UTC calendar day across a clock change', () => {
  const range = statementPeriod(dayjs('2025-03-29'), dayjs('2025-03-30'));
  expect(range?.from.toISOString()).toBe('2025-03-29T00:00:00.000Z');
  expect(range?.until.toISOString()).toBe('2025-03-31T00:00:00.000Z');
  expect(statementPeriod(dayjs('2025-03-29'), dayjs('2025-03-28'))).toBeNull();
  expect(statementPeriod(null, dayjs('2025-03-30'))).toBeNull();
});

it.each([
  { admin: false, basis: 'created', path: '/v1/me/wallets/shop/statement' },
  { admin: true, basis: 'paid', path: '/v1/wallets/shop/statement' },
])('downloads a $basis statement from $path', async ({ admin, basis, path }) => {
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response('transaction_id\r\n'));
  client.setConfig({ baseUrl: 'https://statement.test', fetch: fetcher });
  vi.stubGlobal(
    'URL',
    class extends URL {
      static createObjectURL() {
        return 'blob:statement';
      }
      static revokeObjectURL() {}
    }
  );
  const filenames: string[] = [];
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function captureFilename(
    this: HTMLAnchorElement
  ) {
    filenames.push(this.download);
  });
  render(
    <LocalizationProvider dateAdapter={AdapterDayjs}>
      <WalletStatementDownload walletId="shop" admin={admin} />
    </LocalizationProvider>
  );
  fireEvent.click(screen.getByRole('button', { name: 'statement.action' }));
  expect(screen.getByRole('radio', { name: 'statement.created' })).toBeChecked();
  if (basis === 'paid') fireEvent.click(screen.getByRole('radio', { name: 'statement.paid' }));
  fireEvent.click(screen.getByRole('button', { name: 'statement.csv' }));
  await waitFor(() => expect(filenames).toHaveLength(1));
  const url = new URL((fetcher.mock.calls[0][0] as Request).url);
  expect(url.pathname).toBe(path);
  expect(url.searchParams.get('date_basis')).toBe(basis);
  expect(url.searchParams.get('from')).toMatch(/T00:00:00\.000Z$/);
  expect(url.searchParams.get('until')).toMatch(/T00:00:00\.000Z$/);
  expect(filenames[0]).toMatch(basis === 'paid' ? /-paid\.csv$/ : /\d\.csv$/);
});
