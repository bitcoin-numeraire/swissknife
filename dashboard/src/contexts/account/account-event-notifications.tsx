'use client';

import type { ExternalToast } from 'src/components/snackbar';
import type { Wallet, Account, ClientEvent } from 'src/lib/swissknife';

import { useRef, useEffect } from 'react';

import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';

import { paths } from 'src/routes/paths';
import { RouterLink } from 'src/routes/components';

import { useTranslate } from 'src/locales';
import { ClientEventType } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';
import { toast, snackbarClasses } from 'src/components/snackbar';

import { dashboardSettings } from './account-selection';
import { notificationPreferences } from './notification-preferences';

const eventKinds = {
  [ClientEventType.PAYMENT_ACCEPTED]: undefined,
  [ClientEventType.PAYMENT_PROCESSING]: undefined,
  [ClientEventType.WITHDRAWAL_PROCESSING]: undefined,
  [ClientEventType.WITHDRAWAL_WAITING]: undefined,
  [ClientEventType.WITHDRAWAL_RECONCILING]: undefined,
  [ClientEventType.WITHDRAWAL_BROADCAST]: undefined,
  [ClientEventType.INVOICE_PENDING]: 'pending',
  [ClientEventType.INVOICE_PAID]: 'received',
  [ClientEventType.PAYMENT_SETTLED]: 'sent',
  [ClientEventType.PAYMENT_FAILED]: 'failed',
} as const;

export function accountEventNotification(event: ClientEvent, wallets: readonly Wallet[]) {
  const kind = eventKinds[event.event_type];
  if (!kind) return undefined;

  const incoming = kind === 'pending' || kind === 'received';
  const amount =
    (kind === 'received' ? event.data.amount_received_msat : null) ?? event.data.amount_msat;
  const wallet = wallets.find((item) => item.id === event.wallet_id);

  return {
    kind,
    incoming,
    amountMsat:
      typeof amount === 'number' && Number.isSafeInteger(amount) && amount >= 0
        ? amount
        : undefined,
    walletName:
      wallet?.label || wallet?.asset?.name || wallet?.asset?.display_ticker || event.wallet_id,
    rail: typeof event.data.ledger === 'string' ? event.data.ledger : undefined,
    description:
      typeof event.data.description === 'string' ? event.data.description.trim() : undefined,
    href: incoming
      ? paths.wallet.invoice(event.resource_id, event.wallet_id)
      : paths.wallet.payment(event.resource_id, event.wallet_id),
    toastId: `account-event-${event.wallet_id}-${event.resource_id}`,
  };
}

export function collectUnseenNotifications(
  events: readonly ClientEvent[],
  wallets: readonly Wallet[],
  seen: Set<string>,
  preferences: ReturnType<typeof notificationPreferences>
) {
  return events.flatMap((event) => {
    if (seen.has(event.id)) return [];
    seen.add(event.id);
    const notification = accountEventNotification(event, wallets);
    return notification && preferences[notification.incoming ? 'incoming' : 'outgoing']
      ? [notification]
      : [];
  });
}

export function formatEventSats(msat: number, locale: string) {
  const exact = BigInt(msat);
  const decimal =
    `${exact / BigInt(1000)}.${(exact % BigInt(1000)).toString().padStart(3, '0')}` as `${number}`;
  return Intl.NumberFormat(locale, { maximumFractionDigits: 3 }).format(decimal);
}

function playNotificationSound() {
  try {
    if (!window.AudioContext) return;
    const context = new AudioContext();
    if (context.state !== 'running') {
      void context.close().catch(() => {});
      return;
    }
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.connect(gain).connect(context.destination);
    oscillator.frequency.value = 660;
    gain.gain.setValueAtTime(0.04, context.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, context.currentTime + 0.14);
    oscillator.onended = () => {
      void context.close().catch(() => {});
    };
    oscillator.start();
    oscillator.stop(context.currentTime + 0.15);
  } catch {
    // Audio is optional when a device or its browser disallows playback.
  }
}

export function useAccountEventNotifications(
  account: Account | undefined,
  events: readonly ClientEvent[],
  wallets: readonly Wallet[]
) {
  const { t, currentLang } = useTranslate();
  const seen = useRef(new Set<string>());
  const active = useRef(new Set<string>());
  const accountId = account?.id;
  const settings = account?.preferences?.dashboard_settings;

  useEffect(() => {
    const activeToasts = active.current;
    seen.current = new Set();
    return () => {
      activeToasts.forEach((id) => toast.dismiss(id));
      activeToasts.clear();
    };
  }, [accountId]);

  useEffect(() => {
    if (!accountId) return;
    const preferences = notificationPreferences(settings);
    const hideAmounts = dashboardSettings(settings).hide_balances === true;
    const notifications = collectUnseenNotifications(events, wallets, seen.current, preferences);
    for (const notification of notifications) {
      const { kind, amountMsat, toastId, rail } = notification;
      const title =
        hideAmounts || amountMsat == null
          ? t(`notification_inbox.${kind}`)
          : t(`event_notifications.${kind}_title`, {
              amount: formatEventSats(amountMsat, currentLang.numberFormat.code),
            });
      const description = [
        notification.walletName,
        rail && t(`event_notifications.rail.${rail.toLowerCase()}`, { defaultValue: rail }),
        notification.description,
      ]
        .filter(Boolean)
        .join(' · ');
      active.current.add(toastId);
      const options: ExternalToast = {
        id: toastId,
        description,
        duration: 12_000,
        position: 'top-center',
        className: snackbarClasses.payment,
        onDismiss: () => active.current.delete(toastId),
        onAutoClose: () => active.current.delete(toastId),
        action: (
          <Tooltip title={t('event_notifications.see_details')}>
            <IconButton
              component={RouterLink}
              href={notification.href}
              aria-label={t('event_notifications.see_details')}
              onClick={() => toast.dismiss(toastId)}
              sx={{ mr: 3, flexShrink: 0 }}
            >
              <Iconify icon="solar:eye-bold" />
            </IconButton>
          </Tooltip>
        ),
      };
      if (kind === 'failed') toast.error(title, options);
      else if (kind === 'pending') toast.info(title, options);
      else toast.success(title, options);
    }
    if (preferences.sound && notifications.length) playNotificationSound();
  }, [accountId, currentLang.numberFormat.code, events, settings, t, wallets]);
}
