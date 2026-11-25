import type { Wallet, ClientEvent } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import { ClientEventType } from 'src/lib/swissknife';

import { notificationPreferences } from './notification-preferences';
import {
  formatEventSats,
  accountEventNotification,
  collectUnseenNotifications,
} from './account-event-notifications';

const wallet = { id: 'wallet-1', label: 'Daily wallet' } as Wallet;
function event(
  id: string,
  type: ClientEvent['event_type'] = ClientEventType.INVOICE_PENDING
): ClientEvent {
  return {
    id,
    event_type: type,
    wallet_id: wallet.id,
    resource_id: 'transaction-1',
    created_at: new Date(),
    data: { amount_msat: 21_000, ledger: 'Onchain', description: 'Order 42' },
  };
}

describe('live account notices', () => {
  it('uses the settlement amount and opens activity in its originating wallet', () => {
    const received = event('2', ClientEventType.INVOICE_PAID);
    received.data.amount_received_msat = 32_001;
    expect(accountEventNotification(received, [wallet])).toMatchObject({
      kind: 'received',
      amountMsat: 32_001,
      walletName: 'Daily wallet',
      rail: 'Onchain',
      description: 'Order 42',
      href: '/wallet/invoices?id=transaction-1&wallet_id=wallet-1',
    });
    expect(
      accountEventNotification(event('3', ClientEventType.PAYMENT_FAILED), [wallet])
    ).toMatchObject({
      kind: 'failed',
      incoming: false,
      href: '/wallet/payments?id=transaction-1&wallet_id=wallet-1',
    });
  });

  it('uses exact sub-satoshi precision and omits unsafe event amounts', () => {
    expect(formatEventSats(9_007_199_254_740_991, 'en-US')).toBe('9,007,199,254,740.991');
    expect(formatEventSats(21_001, 'de-DE')).toBe('21,001');
    const unsafe = event('5');
    unsafe.data.amount_msat = Number.MAX_SAFE_INTEGER + 1;
    expect(accountEventNotification(unsafe, [wallet])?.amountMsat).toBeUndefined();
  });

  it('deduplicates replay while allowing confirmation and suppresses muted events even after re-enabling alerts', () => {
    const seen = new Set<string>();
    const events = [
      event('1'),
      event('2', ClientEventType.INVOICE_PAID),
      event('3', ClientEventType.PAYMENT_SETTLED),
    ];
    const saved = notificationPreferences({
      notifications: { incoming: false, outgoing: true, sound: true },
    });
    expect(
      collectUnseenNotifications(events, [wallet], seen, saved).map((notice) => notice.kind)
    ).toEqual(['sent']);
    expect(collectUnseenNotifications(events, [wallet], seen, notificationPreferences({}))).toEqual(
      []
    );
    expect(
      collectUnseenNotifications(events, [wallet], new Set(), notificationPreferences({}))
    ).toHaveLength(3);
    expect(notificationPreferences({ notifications: { incoming: 'invalid' } })).toEqual({
      incoming: true,
      outgoing: true,
      sound: false,
    });
  });
});
