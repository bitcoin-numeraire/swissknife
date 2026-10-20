import { decode } from 'light-bolt11-decoder';
import { it, vi, expect, describe, afterEach, beforeEach } from 'vitest';
import { act, render, screen, waitFor, fireEvent } from '@testing-library/react';

import { ThemeProvider } from '@mui/material/styles';

import { createTheme } from 'src/theme/create-theme';
import {
  pay,
  Ledger,
  walletPay,
  type Payment,
  PaymentStatus,
  estimatePaymentFee,
  estimateWalletPaymentFee,
} from 'src/lib/swissknife';

import { SendMoneyDrawer } from './money-drawers';

vi.mock('src/routes/hooks', () => ({ useRouter: () => ({ push: vi.fn() }) }));
vi.mock('src/locales', () => ({
  useTranslate: () => ({ t: (key: string) => key }),
  formatNumberLocale: () => 'en-US',
}));
vi.mock('src/components/settings', () => ({
  useSettingsContext: () => ({ state: { currency: 'USD', displayUnit: 'sats' } }),
}));
vi.mock('src/actions/account-wallet', () => ({
  useActiveWallet: () => ({ wallet: { id: 'wallet-1' } }),
}));
vi.mock(import('src/lib/swissknife'), async (importOriginal) => ({
  ...(await importOriginal()),
  pay: vi.fn(),
  walletPay: vi.fn(),
  estimatePaymentFee: vi.fn(),
  estimateWalletPaymentFee: vi.fn(),
}));

// Public decoding vector from light-bolt11-decoder 3.2.0 tests/basic.test.js.
const invoice =
  'lnbc20u1p3y0x3hpp5743k2g0fsqqxj7n8qzuhns5gmkk4djeejk3wkp64ppevgekvc0jsdqcve5kzar2v9nr5gpqd4hkuetesp5ez2g297jduwc20t6lmqlsg3man0vf2jfd8ar9fh8fhn2g8yttfkqxqy9gcqcqzys9qrsgqrzjqtx3k77yrrav9hye7zar2rtqlfkytl094dsp0ms5majzth6gt7ca6uhdkxl983uywgqqqqlgqqqvx5qqjqrzjqd98kxkpyw0l9tyy8r8q57k7zpy9zjmh6sez752wj6gcumqnj3yxzhdsmg6qq56utgqqqqqqqqqqqeqqjq7jd56882gtxhrjm03c93aacyfy306m4fq0tskf83c0nmet8zc2lxyyg3saz8x6vwcp26xnrlagf9semau3qm2glysp7sv95693fphvsp54l567';
const address = '1BoatSLRHtKNngkdXEeobR76b53LETtpyT';
const unified = `bitcoin:${address}?amount=0.0005&lightning=${invoice}`;
const invoiceDetails = decode(invoice) as {
  sections: { name: string; value: unknown }[];
  expiry: number;
};
const invoiceTimestamp = Number(
  invoiceDetails.sections.find((section) => section.name === 'timestamp')?.value
);
const invoiceExpiryMs = (invoiceTimestamp + invoiceDetails.expiry) * 1000;

function renderSendDrawer(input = unified, isAdmin = false) {
  return render(
    <ThemeProvider theme={createTheme()}>
      <SendMoneyDrawer
        open
        isAdmin={isAdmin}
        walletId="wallet-1"
        balance={100_000_000}
        contacts={[]}
        fiatPrices={{ USD: 100_000 }}
        initialInput={input}
        onClose={vi.fn()}
      />
    </ThemeProvider>
  );
}

describe('SendMoneyDrawer expired Unified requests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-10-19T00:00:00Z'));
    const quote = {
      ledger: Ledger.ONCHAIN,
      amount_msat: 50_000_000,
      estimated_fee_msat: 1_000,
      maximum_fee_msat: 1_000,
      estimated_total_msat: 50_001_000,
      maximum_total_msat: 50_001_000,
    };
    vi.mocked(estimateWalletPaymentFee).mockResolvedValue(response(quote));
    vi.mocked(estimatePaymentFee).mockResolvedValue(response(quote));
    const payment = {
      id: 'payment-1',
      amount_msat: 50_000_000,
      ledger: Ledger.ONCHAIN,
      status: PaymentStatus.PENDING,
    } as Payment;
    vi.mocked(walletPay).mockResolvedValue(response(payment));
    vi.mocked(pay).mockResolvedValue(response(payment));
  });

  afterEach(() => vi.useRealTimers());

  it('blocks quote and send until the payer explicitly selects an on-chain request', () => {
    renderSendDrawer();

    expect(screen.getByText('send_money.invoice_fallback_expired_note')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'send_money.estimate_fee' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'send_money.send' })).toBeDisabled();
  });

  it.each([false, true])(
    'quotes and sends only the explicitly selected on-chain URI and its own amount (admin: %s)',
    async (isAdmin) => {
      renderSendDrawer(unified, isAdmin);

      fireEvent.click(screen.getByRole('button', { name: 'send_money.use_onchain' }));

      const onchain = `bitcoin:${address}?amount=0.0005`;
      expect(screen.getByPlaceholderText('send_money.recipient_placeholder')).toHaveValue(onchain);
      const quote = isAdmin ? estimatePaymentFee : estimateWalletPaymentFee;
      const send = isAdmin ? pay : walletPay;
      expect(send).not.toHaveBeenCalled();
      expect(quote).not.toHaveBeenCalled();
      fireEvent.click(screen.getByRole('button', { name: 'send_money.estimate_fee' }));
      await waitFor(() => expect(quote).toHaveBeenCalledOnce());
      expect(vi.mocked(quote).mock.calls[0][0]).toMatchObject({
        ...(!isAdmin && { path: { wallet_id: 'wallet-1' } }),
        body: {
          input: onchain,
          amount_msat: 50_000_000,
          comment: null,
          ...(isAdmin && { wallet_id: 'wallet-1' }),
        },
      });
      fireEvent.click(screen.getByRole('button', { name: 'send_money.send_onchain' }));
      await waitFor(() => expect(send).toHaveBeenCalledOnce());
      expect(vi.mocked(send).mock.calls[0][0]).toEqual({
        ...(!isAdmin && { path: { wallet_id: 'wallet-1' } }),
        body: {
          input: onchain,
          amount_msat: 50_000_000,
          comment: null,
          ...(isAdmin && { wallet_id: 'wallet-1' }),
        },
      });
    }
  );

  it('requires a fresh amount when the selected on-chain request has none', async () => {
    renderSendDrawer(`bitcoin:${address}`);
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '2000' } });
    fireEvent.change(screen.getByPlaceholderText('send_money.recipient_placeholder'), {
      target: { value: `bitcoin:${address}?lightning=${invoice}&message=Order%20%2342` },
    });

    fireEvent.click(screen.getByRole('button', { name: 'send_money.use_onchain' }));

    expect(screen.getByRole('spinbutton')).toHaveValue(null);
    expect(screen.getByRole('button', { name: 'send_money.send_onchain' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'send_money.estimate_fee' })).toBeDisabled();
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '50000' } });
    fireEvent.click(screen.getByRole('button', { name: 'send_money.send_onchain' }));
    await waitFor(() => expect(walletPay).toHaveBeenCalledOnce());
    expect(vi.mocked(walletPay).mock.calls[0][0]?.body).toEqual({
      input: `bitcoin:${address}?message=Order%20%2342`,
      amount_msat: 50_000_000,
      comment: null,
    });
  });

  it('keeps standalone expired invoices blocked without offering another rail', () => {
    renderSendDrawer(invoice);

    expect(screen.getByText('send_money.invoice_expired_note')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'send_money.send' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'send_money.estimate_fee' })).toBeDisabled();
    expect(
      screen.queryByRole('button', { name: 'send_money.use_onchain' })
    ).not.toBeInTheDocument();
  });

  it.each([
    ['invalid amount', `bitcoin:${address}?amount=0.000000001&lightning=${invoice}`],
    ['required parameter', `bitcoin:${address}?amount=0.0005&lightning=${invoice}&req-feature=1`],
    [
      'invalid Bech32 checksum',
      `bitcoin:bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080?amount=0.0005&lightning=${invoice}`,
    ],
  ])('does not offer an on-chain request with %s', (_reason, input) => {
    renderSendDrawer(input);

    expect(screen.getByRole('button', { name: 'send_money.send' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'send_money.estimate_fee' })).toBeDisabled();
    expect(
      screen.queryByRole('button', { name: 'send_money.use_onchain' })
    ).not.toBeInTheDocument();
  });

  it('preserves Lightning preference and amount for unexpired Unified requests', async () => {
    vi.setSystemTime(new Date((invoiceTimestamp + 1) * 1000));
    renderSendDrawer();

    expect(
      screen.queryByRole('button', { name: 'send_money.use_onchain' })
    ).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'send_money.estimate_fee' })).toBeEnabled();
    fireEvent.click(screen.getByRole('button', { name: 'send_money.send' }));
    await waitFor(() => expect(walletPay).toHaveBeenCalledOnce());
    expect(vi.mocked(walletPay).mock.calls[0][0]?.body).toEqual({
      input: unified,
      amount_msat: 2_000_000,
      comment: null,
    });
  });

  it('invalidates the previous quote when selecting on-chain after expiry while open', async () => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(new Date(invoiceExpiryMs - 1000));
    renderSendDrawer();
    fireEvent.click(screen.getByRole('button', { name: 'send_money.estimate_fee' }));
    await waitFor(() => expect(estimateWalletPaymentFee).toHaveBeenCalledOnce());
    expect(
      screen.queryByRole('button', { name: 'send_money.estimate_fee' })
    ).not.toBeInTheDocument();

    act(() => vi.advanceTimersByTime(30_000));

    expect(screen.getByRole('button', { name: 'send_money.send' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'send_money.use_onchain' }));
    expect(screen.getByRole('button', { name: 'send_money.estimate_fee' })).toBeEnabled();
    expect(screen.queryByText('send_money.onchain_estimate_disclaimer')).not.toBeInTheDocument();
    expect(estimateWalletPaymentFee).toHaveBeenCalledOnce();
    expect(walletPay).not.toHaveBeenCalled();
  });

  it('does not infer on-chain consent from an undecodable Lightning parameter', () => {
    const input = `bitcoin:${address}?amount=0.0005&lightning=not-an-invoice`;
    renderSendDrawer(input);

    expect(screen.getByPlaceholderText('send_money.recipient_placeholder')).toHaveValue(input);
    expect(
      screen.queryByRole('button', { name: 'send_money.use_onchain' })
    ).not.toBeInTheDocument();
    expect(walletPay).not.toHaveBeenCalled();
  });
});

function response<T>(data: T) {
  return {
    data,
    error: undefined,
    request: new Request('http://localhost'),
    response: new Response(),
  };
}
