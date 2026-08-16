'use client';

import type { ClientEvent } from 'src/lib/swissknife';

import { useSWRConfig } from 'swr';
import { useState, useEffect } from 'react';

import { endpointKeys } from 'src/actions/keys';
import { streamAccountEvents } from 'src/lib/swissknife';

const GLOBAL_EVENT_KEYS = new Set<unknown>([
  endpointKeys.account.get,
  endpointKeys.wallets.listOverviews,
  endpointKeys.invoices.list,
  endpointKeys.payments.list,
  endpointKeys.accountWallet.btcAddresses.list,
]);

const WALLET_EVENT_KEY_PREFIXES = new Set([
  'accountWallet',
  'accountWalletBalance',
  'accountWalletInvoices',
  'accountWalletInvoice',
  'accountWalletPayments',
  'accountWalletPayment',
  'accountWalletContacts',
  'getWallet',
]);

const STREAM_RECONNECT_DELAY_MS = 1_000;

type AccountEventStreamConsumerOptions = {
  signal: AbortSignal;
  openStream: (
    lastEventId: string | undefined,
    onCursorExpired: () => void
  ) => Promise<AsyncIterable<ClientEvent>>;
  onEvent: (event: ClientEvent) => Promise<unknown>;
  onCursorReset?: () => Promise<unknown>;
  onError?: (error: unknown) => void;
  waitForReconnect?: (signal: AbortSignal) => Promise<void>;
};

export function isAccountEventCacheKey(key: unknown, event?: ClientEvent) {
  if (GLOBAL_EVENT_KEYS.has(key)) return true;

  return (
    Array.isArray(key) &&
    typeof key[0] === 'string' &&
    WALLET_EVENT_KEY_PREFIXES.has(key[0]) &&
    (!event || key[1] === event.wallet_id)
  );
}

export function createAccountEventFetch(
  signal: AbortSignal,
  onOpen: () => Promise<unknown>,
  onCursorExpired: () => Promise<unknown>,
  fetchImpl: typeof fetch = globalThis.fetch
): typeof fetch {
  return async (input, init) => {
    const response = await fetchImpl(input, init);

    if (response.status === 409 && !signal.aborted) {
      await response.body?.cancel();
      await onCursorExpired();

      // Turn the handled reset response into a clean EOF. The generated SSE
      // parser would otherwise retry the stale Last-Event-ID forever.
      return new Response('', {
        status: 200,
        headers: { 'content-type': 'text/event-stream' },
      });
    }

    if (response.ok && !signal.aborted) {
      await onOpen();
    }

    return response;
  };
}

function reconnectDelay(signal: AbortSignal) {
  return new Promise<void>((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }

    const onAbort = () => {
      clearTimeout(timeout);
      resolve();
    };
    const timeout = setTimeout(() => {
      signal.removeEventListener('abort', onAbort);
      resolve();
    }, STREAM_RECONNECT_DELAY_MS);

    signal.addEventListener('abort', onAbort, { once: true });
  });
}

export async function consumeAccountEventStreams({
  signal,
  openStream,
  onEvent,
  onCursorReset,
  onError,
  waitForReconnect = reconnectDelay,
}: AccountEventStreamConsumerOptions) {
  let lastEventId: string | undefined;

  while (!signal.aborted) {
    let cursorExpired = false;

    try {
      const stream = await openStream(lastEventId, () => {
        cursorExpired = true;
      });

      for await (const event of stream) {
        if (signal.aborted) return;

        lastEventId = event.id;
        await onEvent(event);
      }
    } catch (error) {
      if (signal.aborted) return;
      onError?.(error);
    }

    if (cursorExpired) {
      lastEventId = undefined;
      await onCursorReset?.();
    }

    if (!signal.aborted) {
      await waitForReconnect(signal);
    }
  }
}

export function useAccountEventStream(accountId: string | undefined, enabled: boolean) {
  const { mutate } = useSWRConfig();
  const [lastEvent, setLastEvent] = useState<{
    accountId: string;
    event: ClientEvent;
  }>();

  useEffect(() => {
    if (!accountId || !enabled) return undefined;

    const controller = new AbortController();
    const refreshCachedState = (event?: ClientEvent) =>
      mutate((key) => isAccountEventCacheKey(key, event));

    void consumeAccountEventStreams({
      signal: controller.signal,
      openStream: async (lastEventId, onCursorExpired) => {
        let cursorExpired = false;
        const { stream } = await streamAccountEvents({
          headers: lastEventId ? { 'Last-Event-ID': lastEventId } : undefined,
          signal: controller.signal,
          fetch: createAccountEventFetch(
            controller.signal,
            () => refreshCachedState(),
            async () => {
              cursorExpired = true;
              onCursorExpired();
            }
          ),
          sseDefaultRetryDelay: 1_000,
          sseMaxRetryDelay: 30_000,
          onSseError: (error) => {
            if (!controller.signal.aborted && !cursorExpired) {
              console.warn('Account event stream disconnected; reconnecting.', error);
            }
          },
        });

        return stream as AsyncIterable<ClientEvent>;
      },
      onEvent: async (clientEvent) => {
        setLastEvent({ accountId, event: clientEvent });
        await refreshCachedState(clientEvent);
      },
      onCursorReset: () => refreshCachedState(),
      onError: (error) => {
        console.warn('Account event stream ended; reconnecting.', error);
      },
    });

    return () => controller.abort();
  }, [accountId, enabled, mutate]);

  if (!enabled || !accountId || !lastEvent || lastEvent.accountId !== accountId) return undefined;
  return lastEvent.event;
}
