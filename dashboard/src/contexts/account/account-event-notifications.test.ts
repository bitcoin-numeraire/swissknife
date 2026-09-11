import type { Wallet, ClientEvent } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import { ClientEventType } from 'src/lib/swissknife';

import {
  invoiceEventNotification,
  collectUnseenInvoiceNotifications,
} from './account-event-notifications';

const wallet = {
  id: 'wallet-1',
  label: 'Daily wallet',
  asset: { name: 'Bitcoin' },
} as Wallet;

function event(
  id: string,
  eventType: ClientEvent['event_type'] = ClientEventType.INVOICE_PENDING
): ClientEvent {
  return {
    id,
    event_type: eventType,
    wallet_id: wallet.id,
    resource_id: 'invoice-1',
    data: {
      amount_msat: 100_000_000,
      amount_received_msat: eventType === ClientEventType.INVOICE_PAID ? 100_000_000 : null,
      ledger: 'Onchain',
      description: 'Order 42',
    },
    created_at: new Date('2026-09-12T08:00:00Z'),
  };
}

describe('invoice event notifications', () => {
  it('builds a rich pending-deposit notification', () => {
    expect(invoiceEventNotification(event('41'), [wallet])).toEqual({
      eventId: '41',
      invoiceId: 'invoice-1',
      kind: 'pending',
      amountMsat: 100_000_000,
      walletName: 'Daily wallet',
      rail: 'Onchain',
      description: 'Order 42',
    });
  });

  it('uses the received amount for a settled invoice', () => {
    const received = event('42', ClientEventType.INVOICE_PAID);
    received.data.amount_received_msat = 120_000_000;

    expect(invoiceEventNotification(received, [wallet])).toMatchObject({
      kind: 'received',
      amountMsat: 120_000_000,
    });
  });

  it('deduplicates replayed event IDs while allowing the confirmation event', () => {
    const seen = new Set<string>();
    const pending = event('41');
    const received = event('42', ClientEventType.INVOICE_PAID);

    expect(collectUnseenInvoiceNotifications([pending], [wallet], seen)).toHaveLength(1);
    expect(collectUnseenInvoiceNotifications([pending], [wallet], seen)).toHaveLength(0);
    expect(collectUnseenInvoiceNotifications([pending, received], [wallet], seen)).toHaveLength(1);
  });

  it('ignores unrelated events and malformed invoice amounts', () => {
    expect(
      invoiceEventNotification({ ...event('43'), event_type: ClientEventType.PAYMENT_SETTLED }, [
        wallet,
      ])
    ).toBeUndefined();

    const malformed = event('44');
    malformed.data.amount_msat = '100000000';
    expect(invoiceEventNotification(malformed, [wallet])).toBeUndefined();
  });
});
