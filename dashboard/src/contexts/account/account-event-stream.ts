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

export function isWalletEventCacheKey(key: unknown, walletId: string) {
  if (GLOBAL_EVENT_KEYS.has(key)) return true;

  return (
    Array.isArray(key) &&
    typeof key[0] === 'string' &&
    WALLET_EVENT_KEY_PREFIXES.has(key[0]) &&
    key[1] === walletId
  );
}

export function useAccountEventStream(walletId: string | undefined, enabled: boolean) {
  const { mutate } = useSWRConfig();

  useEffect(() => {
    if (!walletId || !enabled) return undefined;

    const controller = new AbortController();

    const consume = async () => {
      const { stream } = await streamWalletEvents({
        path: { wallet_id: walletId },
        signal: controller.signal,
        sseDefaultRetryDelay: 1_000,
        sseMaxRetryDelay: 30_000,
        onSseError: (error) => {
          if (!controller.signal.aborted) {
            console.warn('Wallet event stream disconnected; reconnecting.', error);
          }
        },
      });

      for await (const event of stream) {
        const clientEvent = event as ClientEvent;
        if (controller.signal.aborted || clientEvent.wallet_id !== walletId) continue;

        await mutate((key) => isWalletEventCacheKey(key, walletId));
      }
    };

    void consume().catch((error) => {
      if (!controller.signal.aborted) {
        console.warn('Wallet event stream stopped.', error);
      }
    });

    return () => controller.abort();
  }, [enabled, mutate, walletId]);
}
