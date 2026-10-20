import type * as SharedHooks from 'minimal-shared/hooks';

import { it, vi, expect, describe, beforeEach } from 'vitest';
import { act, render, screen, fireEvent } from '@testing-library/react';

import { ThemeProvider } from '@mui/material/styles';

import { composeBip21 } from 'src/utils/bitcoin-request';

import { createTheme } from 'src/theme/create-theme';
import {
  Ledger,
  type Invoice,
  InvoiceStatus,
  BtcAddressType,
  type BtcAddress,
  ClientEventType,
  newWalletInvoice,
  type ClientEvent,
} from 'src/lib/swissknife';

import { getReceiveAddressListState } from './receive-address-list';
import {
  ReceiveMoneyDrawer,
  invoiceAfterClientEvent,
  receivePaymentSuccessFromInvoice,
  receivePaymentSuccessAfterClientEvent,
} from './money-drawers';

const receiveMocks = vi.hoisted(() => ({
  events: [],
  copy: vi.fn(),
  mutateAddresses: vi.fn(),
}));

vi.mock('src/routes/hooks', () => ({ useRouter: () => ({ push: vi.fn() }) }));
vi.mock('src/locales', () => ({
  useTranslate: () => ({ t: (key: string) => key }),
  formatNumberLocale: () => 'en-US',
}));
vi.mock('src/contexts/account', () => ({
  useAccountContext: () => ({ recentClientEvents: receiveMocks.events }),
}));
vi.mock('src/components/settings', () => ({
  useSettingsContext: () => ({ state: { currency: 'USD', displayUnit: 'sats' } }),
}));
vi.mock('src/actions/account-wallet', () => ({
  useActiveWallet: () => ({ wallet: { id: 'wallet-1' } }),
  useGetWalletInvoice: () => ({ invoice: undefined }),
  useListWalletBtcAddresses: () => ({
    btcAddresses: [receiveAddress],
    btcAddressesMutate: receiveMocks.mutateAddresses,
  }),
}));
vi.mock('src/actions/btc-addresses', () => ({
  useListBtcAddresses: () => ({ btcAddresses: [], btcAddressesMutate: vi.fn() }),
}));
vi.mock(import('src/lib/swissknife'), async (importOriginal) => ({
  ...(await importOriginal()),
  newWalletInvoice: vi.fn(),
}));
vi.mock('react-qrcode-logo', () => ({
  QRCode: ({ value }: { value: string }) => <output data-testid="receive-qr">{value}</output>,
}));
vi.mock('minimal-shared/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof SharedHooks>()),
  useCopyToClipboard: () => ({ copy: receiveMocks.copy }),
}));

const receiveAddress = {
  id: 'address-1',
  wallet_id: 'wallet-1',
  address: 'bcrt1qreceive-test',
  address_type: BtcAddressType.P2TR,
  used: false,
  created_at: new Date('2026-10-18T00:00:00Z'),
} as BtcAddress;

const receiveInvoice = {
  id: 'invoice-1',
  wallet_id: 'wallet-1',
  ledger: Ledger.LIGHTNING,
  status: InvoiceStatus.PENDING,
  amount_msat: 1_000_000,
  description: '',
  ln_invoice: { bolt11: 'lnbcrt10u-test-request' },
} as Invoice;

function renderReceiveDrawer() {
  return render(
    <ThemeProvider theme={createTheme()}>
      <ReceiveMoneyDrawer
        open
        walletId="wallet-1"
        fiatPrices={{ USD: 100_000 }}
        onClose={vi.fn()}
      />
    </ThemeProvider>
  );
}

function invoiceResponse(invoice = receiveInvoice) {
  return {
    data: invoice,
    error: undefined,
    request: new Request('http://localhost'),
    response: new Response(),
  };
}

async function generateReceiveRequest() {
  fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '1000' } });
  fireEvent.click(screen.getByRole('button', { name: 'receive_money.generate_invoice' }));
  const qr = await screen.findByTestId('receive-qr');
  expect(qr.textContent).toBe(
    composeBip21(receiveAddress.address, receiveInvoice.ln_invoice?.bolt11, 1000)
  );
}

function expectNoInvoicePayload() {
  expect(screen.queryByTestId('receive-qr')).not.toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: 'receive_money.copy_unified' })
  ).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'share' })).not.toBeInTheDocument();
}

describe('ReceiveMoneyDrawer request coherence', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(newWalletInvoice).mockResolvedValue(invoiceResponse());
  });

  it('hides the generated QR and copy/share payload after the amount changes', async () => {
    renderReceiveDrawer();
    await generateReceiveRequest();

    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '50000' } });

    expectNoInvoicePayload();
    expect(newWalletInvoice).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'receive_money.refresh_request' })).toBeEnabled();
  });

  it.each(['unified', 'lightning'])('hides a stale %s request after a memo edit', async (mode) => {
    renderReceiveDrawer();
    await generateReceiveRequest();
    fireEvent.click(screen.getByRole('button', { name: `receive_money.${mode}` }));

    fireEvent.change(screen.getByLabelText('receive_money.memo'), {
      target: { value: 'Order 42' },
    });

    expectNoInvoicePayload();
    expect(
      screen.queryByRole('button', { name: 'receive_money.copy_lightning' })
    ).not.toBeInTheDocument();
    expect(newWalletInvoice).toHaveBeenCalledOnce();
  });

  it('hides the generated request after an expiry edit', async () => {
    renderReceiveDrawer();
    await generateReceiveRequest();

    fireEvent.click(screen.getByRole('button', { name: 'receive_money.expiry_1_day' }));

    expectNoInvoicePayload();
    expect(newWalletInvoice).toHaveBeenCalledOnce();
  });

  it('regenerates a coherent request for the edited amount on explicit refresh', async () => {
    renderReceiveDrawer();
    await generateReceiveRequest();
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '50000' } });
    expectNoInvoicePayload();
    const refreshed = {
      ...receiveInvoice,
      id: 'invoice-2',
      amount_msat: 50_000_000,
      ln_invoice: { ...receiveInvoice.ln_invoice!, bolt11: 'lnbcrt500u-refreshed-test-request' },
    };
    vi.mocked(newWalletInvoice).mockResolvedValueOnce(invoiceResponse(refreshed));

    fireEvent.click(screen.getByRole('button', { name: 'receive_money.refresh_request' }));

    const qr = await screen.findByTestId('receive-qr');
    const payload = composeBip21(receiveAddress.address, refreshed.ln_invoice.bolt11, 50000);
    expect(qr.textContent).toBe(payload);
    expect(vi.mocked(newWalletInvoice).mock.calls[1][0]?.body).toEqual({
      amount_msat: 50_000_000,
      description: '',
      expiry: 3600,
    });
    fireEvent.click(screen.getByRole('button', { name: 'receive_money.copy_unified' }));
    expect(receiveMocks.copy).toHaveBeenCalledWith(payload);
  });

  it('does not expose a stale response when the form changes during generation', async () => {
    let finish!: (response: ReturnType<typeof invoiceResponse>) => void;
    vi.mocked(newWalletInvoice).mockImplementationOnce(
      () =>
        new Promise<ReturnType<typeof invoiceResponse>>((resolve) => {
          finish = resolve;
        })
    );
    renderReceiveDrawer();
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '1000' } });
    fireEvent.click(screen.getByRole('button', { name: 'receive_money.generate_invoice' }));
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '50000' } });

    await act(async () => {
      finish(invoiceResponse());
    });

    expectNoInvoicePayload();
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '1000' } });
    expect(screen.getByTestId('receive-qr').textContent).toBe(
      composeBip21(receiveAddress.address, receiveInvoice.ln_invoice?.bolt11, 1000)
    );
  });

  it('preserves amountless unified requests until an amount is entered', async () => {
    const amountless = {
      ...receiveInvoice,
      amount_msat: 0,
      ln_invoice: { ...receiveInvoice.ln_invoice!, bolt11: 'lnbcrt1-amountless-test-request' },
    };
    vi.mocked(newWalletInvoice).mockResolvedValueOnce(invoiceResponse(amountless));
    renderReceiveDrawer();
    fireEvent.click(screen.getByRole('button', { name: 'receive_money.generate_invoice' }));
    expect((await screen.findByTestId('receive-qr')).textContent).toBe(
      composeBip21(receiveAddress.address, amountless.ln_invoice.bolt11)
    );

    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '1000' } });

    expectNoInvoicePayload();
  });

  it('keeps on-chain-only requests editable without generating an invoice', () => {
    renderReceiveDrawer();
    fireEvent.click(screen.getByRole('button', { name: 'receive_money.bitcoin' }));
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '50000' } });

    expect(screen.getByTestId('receive-qr').textContent).toBe(
      composeBip21(receiveAddress.address, undefined, 50000)
    );
    expect(newWalletInvoice).not.toHaveBeenCalled();
  });
});

describe('getReceiveAddressListState', () => {
  it('uses the account-wallet address list for the regular receive drawer', () => {
    expect(
      getReceiveAddressListState({
        open: true,
        isAdmin: false,
        selectedNeedsAddress: true,
        addressWalletId: 'wallet-1',
      })
    ).toEqual({
      adminQuery: undefined,
      adminEnabled: false,
      walletEnabled: true,
    });
  });

  it('uses the admin address list when receiving for a selected wallet', () => {
    expect(
      getReceiveAddressListState({
        open: true,
        isAdmin: true,
        selectedNeedsAddress: true,
        addressWalletId: 'wallet-1',
      })
    ).toEqual({
      adminQuery: { wallet_id: 'wallet-1' },
      adminEnabled: true,
      walletEnabled: false,
    });
  });

  it('does not fetch addresses while the drawer is closed or address-free payloads are selected', () => {
    expect(
      getReceiveAddressListState({
        open: false,
        isAdmin: false,
        selectedNeedsAddress: true,
      })
    ).toEqual({
      adminQuery: undefined,
      adminEnabled: false,
      walletEnabled: false,
    });

    expect(
      getReceiveAddressListState({
        open: true,
        isAdmin: false,
        selectedNeedsAddress: false,
      })
    ).toEqual({
      adminQuery: undefined,
      adminEnabled: false,
      walletEnabled: false,
    });
  });
});

describe('invoiceAfterClientEvent', () => {
  const invoice = {
    id: 'invoice-1',
    wallet_id: 'wallet-1',
    status: InvoiceStatus.PENDING,
  } as Invoice;
  const paidEvent = {
    id: '41',
    event_type: ClientEventType.INVOICE_PAID,
    wallet_id: 'wallet-1',
    resource_id: 'invoice-1',
    data: { amount_received_msat: 42_000 },
    created_at: new Date('2026-08-16T12:00:00Z'),
  } satisfies ClientEvent;

  it('marks the displayed invoice paid when its durable event arrives', () => {
    expect(invoiceAfterClientEvent(invoice, paidEvent, 'wallet-1')).toMatchObject({
      status: InvoiceStatus.SETTLED,
      amount_received_msat: 42_000,
      payment_time: paidEvent.created_at,
    });
  });

  it('ignores events for a different resource or wallet', () => {
    expect(
      invoiceAfterClientEvent(invoice, { ...paidEvent, resource_id: 'invoice-2' }, 'wallet-1')
    ).toBe(invoice);
    expect(invoiceAfterClientEvent(invoice, paidEvent, 'wallet-2')).toBe(invoice);
  });
});

describe('receivePaymentSuccessAfterClientEvent', () => {
  const invoice = {
    id: 'invoice-1',
    wallet_id: 'wallet-1',
    status: InvoiceStatus.PENDING,
    ledger: Ledger.LIGHTNING,
    amount_msat: 100_000_000,
    description: 'Order 42',
  } as Invoice;

  const paidEvent = {
    id: '42',
    event_type: ClientEventType.INVOICE_PAID,
    wallet_id: 'wallet-1',
    resource_id: 'invoice-1',
    data: {
      amount_received_msat: 100_000_000,
      ledger: Ledger.LIGHTNING,
      description: 'Order 42',
    },
    created_at: new Date('2026-09-12T08:00:00Z'),
  } satisfies ClientEvent;

  it('recognizes settlement of the displayed Lightning invoice', () => {
    expect(
      receivePaymentSuccessAfterClientEvent(undefined, invoice, paidEvent, 'wallet-1', undefined)
    ).toEqual({
      invoiceId: 'invoice-1',
      amountMsat: 100_000_000,
      ledger: Ledger.LIGHTNING,
      description: 'Order 42',
    });
  });

  it('recognizes an on-chain payment to the address in a unified request', () => {
    const onchainEvent = {
      ...paidEvent,
      resource_id: 'onchain-invoice-1',
      data: {
        amount_received_msat: 120_000_000,
        ledger: Ledger.ONCHAIN,
        bitcoin_output: { address: 'bcrt1qdisplayed' },
      },
    } satisfies ClientEvent;

    expect(
      receivePaymentSuccessAfterClientEvent(
        undefined,
        invoice,
        onchainEvent,
        'wallet-1',
        'bcrt1qdisplayed'
      )
    ).toEqual({
      invoiceId: 'onchain-invoice-1',
      amountMsat: 120_000_000,
      ledger: Ledger.ONCHAIN,
      description: 'Order 42',
    });
  });

  it('ignores payments for other invoices, addresses, and wallets', () => {
    expect(
      receivePaymentSuccessAfterClientEvent(
        undefined,
        invoice,
        { ...paidEvent, resource_id: 'invoice-2' },
        'wallet-1',
        'bcrt1qdisplayed'
      )
    ).toBeUndefined();
    expect(
      receivePaymentSuccessAfterClientEvent(undefined, invoice, paidEvent, 'wallet-2', undefined)
    ).toBeUndefined();
  });
});

describe('receivePaymentSuccessFromInvoice', () => {
  const invoice = {
    id: 'missed-event-invoice',
    wallet_id: 'wallet-1',
    status: InvoiceStatus.SETTLED,
    ledger: Ledger.LIGHTNING,
    amount_msat: 100_000,
    amount_received_msat: 120_000,
  } as Invoice;

  it('restores the receive success state from REST after a missed settlement event', () => {
    expect(receivePaymentSuccessFromInvoice(invoice, 'wallet-1')).toMatchObject({
      invoiceId: invoice.id,
      amountMsat: 120_000,
      ledger: Ledger.LIGHTNING,
    });
  });

  it('does not claim success for a pending invoice or another wallet', () => {
    expect(
      receivePaymentSuccessFromInvoice({ ...invoice, status: InvoiceStatus.PENDING }, 'wallet-1')
    ).toBeUndefined();
    expect(receivePaymentSuccessFromInvoice(invoice, 'wallet-2')).toBeUndefined();
  });
});
