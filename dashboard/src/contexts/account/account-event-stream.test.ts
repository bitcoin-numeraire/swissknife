import type { ClientEvent } from 'src/lib/swissknife';

import { it, vi, expect, describe } from 'vitest';

import { endpointKeys } from 'src/actions/keys';

import {
  isWalletEventCacheKey,
  createWalletEventFetch,
  consumeWalletEventStreams,
} from './account-event-stream';

describe('wallet event cache invalidation', () => {
  const walletId = 'wallet-1';

  it('refreshes the wallet balance and transaction resources', () => {
    expect(isWalletEventCacheKey(endpointKeys.accountWallet.balance(walletId), walletId)).toBe(
      true
    );
    expect(
      isWalletEventCacheKey(endpointKeys.accountWallet.payments.list(walletId, 25, 0), walletId)
    ).toBe(true);
    expect(
      isWalletEventCacheKey(
        endpointKeys.accountWallet.invoices.get(walletId, 'invoice-1'),
        walletId
      )
    ).toBe(true);
  });

  it('does not refresh another wallet cache', () => {
    expect(isWalletEventCacheKey(endpointKeys.accountWallet.get('wallet-2'), walletId)).toBe(false);
  });

  it('refreshes shared dashboard aggregates', () => {
    expect(isWalletEventCacheKey(endpointKeys.account.get, walletId)).toBe(true);
    expect(isWalletEventCacheKey(endpointKeys.wallets.listOverviews, walletId)).toBe(true);
  });

  it('refreshes wallet state after the event stream cursor is established', async () => {
    const controller = new AbortController();
    const response = new Response(null, { status: 200 });
    const fetchImpl = vi.fn().mockResolvedValue(response) as unknown as typeof fetch;
    const refresh = vi.fn().mockResolvedValue(undefined);

    const eventFetch = createWalletEventFetch(controller.signal, refresh, fetchImpl);

    await expect(eventFetch('https://example.com/events')).resolves.toBe(response);
    expect(refresh).toHaveBeenCalledOnce();
  });

  it('does not refresh after an unsuccessful or cancelled connection', async () => {
    const controller = new AbortController();
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(new Response(null, { status: 401 }))
      .mockResolvedValueOnce(new Response(null, { status: 200 })) as unknown as typeof fetch;
    const refresh = vi.fn().mockResolvedValue(undefined);
    const eventFetch = createWalletEventFetch(controller.signal, refresh, fetchImpl);

    await eventFetch('https://example.com/events');
    controller.abort();
    await eventFetch('https://example.com/events');

    expect(refresh).not.toHaveBeenCalled();
  });

  it('reopens a normally-ended stream from its last durable event ID', async () => {
    const controller = new AbortController();
    const received: Array<string> = [];
    const cursors: Array<string | undefined> = [];

    async function* stream(id: string) {
      yield {
        id,
        event_type: 'payment.settled',
        wallet_id: walletId,
        resource_id: 'payment-1',
        data: {},
        created_at: new Date('2026-08-13T18:00:00Z'),
      } satisfies ClientEvent;
    }

    const openStream = vi.fn(async (lastEventId: string | undefined) => {
      cursors.push(lastEventId);
      return stream(lastEventId ? '42' : '41');
    });

    await consumeWalletEventStreams({
      signal: controller.signal,
      openStream,
      onEvent: async (event) => {
        received.push(event.id);
        if (event.id === '42') controller.abort();
      },
      waitForReconnect: async () => undefined,
    });

    expect(received).toEqual(['41', '42']);
    expect(cursors).toEqual([undefined, '41']);
    expect(openStream).toHaveBeenCalledTimes(2);
  });
});
