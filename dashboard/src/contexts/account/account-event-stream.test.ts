import type { ClientEvent } from 'src/lib/swissknife';

import { it, vi, expect, describe } from 'vitest';

import { endpointKeys } from 'src/actions/keys';

import {
  isAccountEventCacheKey,
  createAccountEventFetch,
  appendRecentClientEvent,
  consumeAccountEventStreams,
} from './account-event-stream';

const walletId = 'wallet-1';

function event(id: string, eventWalletId = walletId): ClientEvent {
  return {
    id,
    event_type: 'payment.settled',
    wallet_id: eventWalletId,
    resource_id: 'payment-1',
    data: {},
    created_at: new Date('2026-08-13T18:00:00Z'),
  };
}

describe('account event cache invalidation', () => {
  it('refreshes the affected wallet balance and transaction resources', () => {
    const clientEvent = event('41');

    expect(isAccountEventCacheKey(endpointKeys.accountWallet.balance(walletId), clientEvent)).toBe(
      true
    );
    expect(
      isAccountEventCacheKey(endpointKeys.accountWallet.payments.list(walletId, 25, 0), clientEvent)
    ).toBe(true);
    expect(
      isAccountEventCacheKey(
        endpointKeys.accountWallet.invoices.get(walletId, 'invoice-1'),
        clientEvent
      )
    ).toBe(true);
  });

  it('does not refresh another wallet cache', () => {
    expect(isAccountEventCacheKey(endpointKeys.accountWallet.get('wallet-2'), event('41'))).toBe(
      false
    );
  });

  it('refreshes shared dashboard aggregates', () => {
    const clientEvent = event('41');

    expect(isAccountEventCacheKey(endpointKeys.account.get, clientEvent)).toBe(true);
    expect(isAccountEventCacheKey(endpointKeys.wallets.listOverviews, clientEvent)).toBe(true);
  });

  it('refreshes every cached account wallet after a connection establishes its cursor', () => {
    expect(isAccountEventCacheKey(endpointKeys.accountWallet.get('wallet-1'))).toBe(true);
    expect(isAccountEventCacheKey(endpointKeys.accountWallet.get('wallet-2'))).toBe(true);
  });
});

describe('recent account events', () => {
  it('preserves burst events for consumers that need an earlier notification', () => {
    const first = appendRecentClientEvent(undefined, 'account-1', event('41'));
    const second = appendRecentClientEvent(first, 'account-1', event('42'));

    expect(second.events.map((clientEvent) => clientEvent.id)).toEqual(['41', '42']);
  });

  it('deduplicates replayed IDs and clears events when the account changes', () => {
    const first = appendRecentClientEvent(undefined, 'account-1', event('41'));
    const replayed = appendRecentClientEvent(first, 'account-1', event('41'));
    const otherAccount = appendRecentClientEvent(replayed, 'account-2', event('42'));

    expect(replayed.events).toHaveLength(1);
    expect(otherAccount.events.map((clientEvent) => clientEvent.id)).toEqual(['42']);
  });
});

describe('account event connection lifecycle', () => {
  it('refreshes REST state after the event stream cursor is established', async () => {
    const controller = new AbortController();
    const response = new Response(null, { status: 200 });
    const fetchImpl = vi.fn().mockResolvedValue(response) as unknown as typeof fetch;
    const refresh = vi.fn().mockResolvedValue(undefined);

    const eventFetch = createAccountEventFetch(controller.signal, refresh, vi.fn(), fetchImpl);

    await expect(eventFetch('https://example.com/events')).resolves.toBe(response);
    expect(refresh).toHaveBeenCalledOnce();
  });

  it('turns an expired cursor into a handled clean stream reset', async () => {
    const controller = new AbortController();
    const response = new Response('{"status":"409 Conflict"}', { status: 409 });
    const fetchImpl = vi.fn().mockResolvedValue(response) as unknown as typeof fetch;
    const reset = vi.fn().mockResolvedValue(undefined);
    const eventFetch = createAccountEventFetch(controller.signal, vi.fn(), reset, fetchImpl);

    const handled = await eventFetch('https://example.com/events');

    expect(handled.status).toBe(200);
    expect(reset).toHaveBeenCalledOnce();
  });

  it('does not refresh after an unsuccessful or cancelled connection', async () => {
    const controller = new AbortController();
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(new Response(null, { status: 401 }))
      .mockResolvedValueOnce(new Response(null, { status: 200 })) as unknown as typeof fetch;
    const refresh = vi.fn().mockResolvedValue(undefined);
    const eventFetch = createAccountEventFetch(controller.signal, refresh, vi.fn(), fetchImpl);

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
      yield event(id);
    }

    const openStream = vi.fn(async (lastEventId: string | undefined) => {
      cursors.push(lastEventId);
      return stream(lastEventId ? '42' : '41');
    });

    await consumeAccountEventStreams({
      signal: controller.signal,
      openStream,
      onEvent: async (clientEvent) => {
        received.push(clientEvent.id);
        if (clientEvent.id === '42') controller.abort();
      },
      waitForReconnect: async () => undefined,
    });

    expect(received).toEqual(['41', '42']);
    expect(cursors).toEqual([undefined, '41']);
    expect(openStream).toHaveBeenCalledTimes(2);
  });

  it('clears a stale cursor, refreshes REST state, and reconnects fresh', async () => {
    const controller = new AbortController();
    const cursors: Array<string | undefined> = [];
    const reset = vi.fn().mockResolvedValue(undefined);

    async function* stream(clientEvent?: ClientEvent) {
      if (clientEvent) yield clientEvent;
    }

    const openStream = vi.fn(
      async (lastEventId: string | undefined, onCursorExpired: () => void) => {
        cursors.push(lastEventId);
        if (cursors.length === 1) return stream(event('41'));
        if (cursors.length === 2) {
          onCursorExpired();
          return stream();
        }
        return stream(event('42'));
      }
    );

    await consumeAccountEventStreams({
      signal: controller.signal,
      openStream,
      onEvent: async (clientEvent) => {
        if (clientEvent.id === '42') controller.abort();
      },
      onCursorReset: reset,
      waitForReconnect: async () => undefined,
    });

    expect(cursors).toEqual([undefined, '41', undefined]);
    expect(reset).toHaveBeenCalledOnce();
  });
});
