'use client';

import type { ClientEvent } from 'src/lib/swissknife';

import { useEffect } from 'react';
import { useSWRConfig } from 'swr';

import { endpointKeys } from 'src/actions/keys';
import { streamWalletEvents } from 'src/lib/swissknife';

const GLOBAL_EVENT_KEYS = new Set<unknown>([
  endpointKeys.account.get,
  endpointKeys.wallets.listOverviews,
  endpointKeys.invoices.list,
  endpointKeys.payments.list,
]);

const WALLET_EVENT_KEY_PREFIXES = new Set([
  'accountWallet',
  'accountWalletBalance',
  'accountWalletInvoices',
  'accountWalletInvoice',
  'accountWalletPayments',
  'accountWalletPayment',
]);

const STREAM_RECONNECT_DELAY_MS = 1_000;

type WalletEventStreamConsumerOptions = {
  signal: AbortSignal;
  openStream: (lastEventId: string | undefined) => Promise<AsyncIterable<ClientEvent>>;
  onEvent: (event: ClientEvent) => Promise<unknown>;
  onError?: (error: unknown) => void;
  waitForReconnect?: (signal: AbortSignal) => Promise<void>;
};

export function isWalletEventCacheKey(key: unknown, walletId: string) {
  if (GLOBAL_EVENT_KEYS.has(key)) return true;

  return (
    Array.isArray(key) &&
    typeof key[0] === 'string' &&
    WALLET_EVENT_KEY_PREFIXES.has(key[0]) &&
    key[1] === walletId
  );
}

export function createWalletEventFetch(
  signal: AbortSignal,
  onOpen: () => Promise<unknown>,
  fetchImpl: typeof fetch = globalThis.fetch
): typeof fetch {
  return async (input, init) => {
    const response = await fetchImpl(input, init);

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

export async function consumeWalletEventStreams({
  signal,
  openStream,
  onEvent,
  onError,
  waitForReconnect = reconnectDelay,
}: WalletEventStreamConsumerOptions) {
  let lastEventId: string | undefined;

  while (!signal.aborted) {
    try {
      const stream = await openStream(lastEventId);

      for await (const event of stream) {
        if (signal.aborted) return;

        lastEventId = event.id;
        await onEvent(event);
      }
    } catch (error) {
      if (signal.aborted) return;
      onError?.(error);
    }

    if (!signal.aborted) {
      await waitForReconnect(signal);
    }
  }
}

export function useAccountEventStream(walletId: string | undefined, enabled: boolean) {
  const { mutate } = useSWRConfig();

  useEffect(() => {
    if (!walletId || !enabled) return undefined;

    const controller = new AbortController();

    void consumeWalletEventStreams({
      signal: controller.signal,
      openStream: async (lastEventId) => {
        const { stream } = await streamWalletEvents({
          path: { wallet_id: walletId },
          headers: lastEventId ? { 'Last-Event-ID': lastEventId } : undefined,
          signal: controller.signal,
          fetch: createWalletEventFetch(controller.signal, () =>
            mutate((key) => isWalletEventCacheKey(key, walletId))
          ),
          sseDefaultRetryDelay: 1_000,
          sseMaxRetryDelay: 30_000,
          onSseError: (error) => {
            if (!controller.signal.aborted) {
              console.warn('Wallet event stream disconnected; reconnecting.', error);
            }
          },
        });

        return stream as AsyncIterable<ClientEvent>;
      },
      onEvent: async (clientEvent) => {
        if (clientEvent.wallet_id === walletId) {
          await mutate((key) => isWalletEventCacheKey(key, walletId));
        }
      },
      onError: (error) => {
        console.warn('Wallet event stream ended; reconnecting.', error);
      },
    });

    return () => controller.abort();
  }, [enabled, mutate, walletId]);
}
