import { it, expect, describe } from 'vitest';

import {
  Ledger,
  type Invoice,
  InvoiceStatus,
  ClientEventType,
  type ClientEvent,
} from 'src/lib/swissknife';

import { getReceiveAddressListState } from './receive-address-list';
import {
  invoiceAfterClientEvent,
  receivePaymentSuccessFromInvoice,
  receivePaymentSuccessAfterClientEvent,
} from './money-drawers';

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
