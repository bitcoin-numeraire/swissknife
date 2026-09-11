'use client';

import type { Wallet, ClientEvent } from 'src/lib/swissknife';

import { useRef, useEffect } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { fSats } from 'src/utils/format-number';

import { useTranslate } from 'src/locales';
import { ClientEventType } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

export type InvoiceEventNotification = {
  eventId: string;
  invoiceId: string;
  kind: 'pending' | 'received';
  amountMsat: number;
  walletName: string;
  rail: string;
  description?: string;
};

function nonEmptyString(value: unknown) {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

function walletName(wallet: Wallet | undefined) {
  return wallet?.label || wallet?.asset?.name || wallet?.asset?.display_ticker || 'Wallet';
}

export function invoiceEventNotification(
  event: ClientEvent,
  wallets: readonly Wallet[]
): InvoiceEventNotification | undefined {
  if (
    event.event_type !== ClientEventType.INVOICE_PENDING &&
    event.event_type !== ClientEventType.INVOICE_PAID
  ) {
    return undefined;
  }

  const kind = event.event_type === ClientEventType.INVOICE_PENDING ? 'pending' : 'received';
  const receivedAmount = event.data.amount_received_msat;
  const requestedAmount = event.data.amount_msat;
  const amountMsat =
    kind === 'received' && typeof receivedAmount === 'number' ? receivedAmount : requestedAmount;

  if (typeof amountMsat !== 'number' || !Number.isFinite(amountMsat) || amountMsat < 0) {
    return undefined;
  }

  return {
    eventId: event.id,
    invoiceId: event.resource_id,
    kind,
    amountMsat,
    walletName: walletName(wallets.find((wallet) => wallet.id === event.wallet_id)),
    rail: nonEmptyString(event.data.ledger) ?? 'Bitcoin',
    description: nonEmptyString(event.data.description),
  };
}

export function collectUnseenInvoiceNotifications(
  events: readonly ClientEvent[],
  wallets: readonly Wallet[],
  seenEventIds: Set<string>
) {
  const notifications: InvoiceEventNotification[] = [];

  for (const event of events) {
    if (seenEventIds.has(event.id)) continue;

    const notification = invoiceEventNotification(event, wallets);
    if (!notification) continue;

    seenEventIds.add(event.id);
    notifications.push(notification);
  }

  return notifications;
}

export function useAccountEventNotifications(
  accountId: string | undefined,
  events: readonly ClientEvent[],
  wallets: readonly Wallet[]
) {
  const router = useRouter();
  const { t } = useTranslate();
  const notificationState = useRef<{ accountId?: string; seenEventIds: Set<string> }>({
    seenEventIds: new Set(),
  });

  useEffect(() => {
    if (!accountId) return;

    if (notificationState.current.accountId !== accountId) {
      notificationState.current = { accountId, seenEventIds: new Set() };
    }

    const notifications = collectUnseenInvoiceNotifications(
      events,
      wallets,
      notificationState.current.seenEventIds
    );

    for (const notification of notifications) {
      const sats = fSats(notification.amountMsat / 1000);
      const title =
        notification.kind === 'pending'
          ? t('event_notifications.incoming_title', { amount: sats })
          : t('event_notifications.received_title', { amount: sats });
      const rail = t(`event_notifications.rail.${notification.rail.toLowerCase()}`, {
        defaultValue: notification.rail,
      });
      const description = [notification.walletName, rail, notification.description]
        .filter(Boolean)
        .join(' · ');
      const options = {
        id: `invoice-event-${notification.invoiceId}`,
        description,
        duration: 12_000,
        action: {
          label: t('event_notifications.view_activity'),
          onClick: () => router.push(paths.activityInvoice(notification.invoiceId)),
        },
      };

      if (notification.kind === 'pending') {
        toast.info(title, options);
      } else {
        toast.success(title, options);
      }
    }
  }, [accountId, events, router, t, wallets]);
}
