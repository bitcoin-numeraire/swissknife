import { it, expect, describe } from 'vitest';

import { type Invoice, InvoiceStatus, ClientEventType, type ClientEvent } from 'src/lib/swissknife';

import { invoiceAfterClientEvent } from './money-drawers';
import { getReceiveAddressListState } from './receive-address-list';

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
    data: {},
    created_at: new Date('2026-08-16T12:00:00Z'),
  } satisfies ClientEvent;

  it('marks the displayed invoice paid when its durable event arrives', () => {
    expect(invoiceAfterClientEvent(invoice, paidEvent, 'wallet-1')).toMatchObject({
      status: InvoiceStatus.SETTLED,
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
