import type { NotificationAction, AccountNotification } from 'src/lib/swissknife';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { paths } from 'src/routes/paths';
import { RouterLink } from 'src/routes/components';

import { useTranslate } from 'src/locales';
import { ClientEventType } from 'src/lib/swissknife';

import { useSettingsContext } from 'src/components/settings';

type Props = {
  item: AccountNotification;
  wallet: string;
  busy: boolean;
  change: (action: NotificationAction) => void;
  onClose: () => void;
};

export function notificationActivity(
  item: Pick<AccountNotification, 'event_type' | 'resource_id' | 'wallet_id'>
) {
  return item.event_type === ClientEventType.INVOICE_PAID
    ? paths.wallet.invoice(item.resource_id, item.wallet_id)
    : paths.wallet.payment(item.resource_id, item.wallet_id);
}

export function NotificationItem({ item, wallet, busy, change, onClose }: Props) {
  const { t, currentLang } = useTranslate();
  const { state } = useSettingsContext();
  const lang = currentLang.numberFormat.code;
  if (
    item.event_type === ClientEventType.PAYMENT_ACCEPTED ||
    item.event_type === ClientEventType.PAYMENT_PROCESSING
  ) {
    return null;
  }
  const title = {
    [ClientEventType.INVOICE_PAID]: 'notification_inbox.received',
    [ClientEventType.PAYMENT_SETTLED]: 'notification_inbox.sent',
    [ClientEventType.PAYMENT_FAILED]: 'notification_inbox.failed',
    [ClientEventType.INVOICE_PENDING]: 'notification_inbox.pending',
  }[item.event_type];
  const amount =
    item.amount_msat == null
      ? undefined
      : state.hideBalances
        ? '••••'
        : t('notification_inbox.amount', {
            amount: Intl.NumberFormat(lang).format(BigInt(item.amount_msat)),
          });
  const time = Intl.DateTimeFormat(lang, { dateStyle: 'medium', timeStyle: 'short' }).format(
    item.created_at
  );

  return (
    <Stack
      component="li"
      spacing={0.75}
      sx={{ py: 1.5, px: 2.5, borderBottom: '1px solid', borderColor: 'divider' }}
    >
      <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
        {!item.is_read && (
          <Box
            aria-label={t('notification_inbox.unread')}
            sx={{ width: 8, height: 8, borderRadius: '50%', bgcolor: 'info.main' }}
          />
        )}
        <Typography variant="subtitle2">{t(title)}</Typography>
        {amount && (
          <Typography variant="subtitle2" sx={{ ml: 'auto !important' }}>
            {amount}
          </Typography>
        )}
      </Stack>
      <Typography variant="body2">
        {wallet}
        {item.ledger && ` · ${t('notification_inbox.rail_' + item.ledger)}`}
      </Typography>
      {item.description && (
        <Typography variant="body2" sx={{ overflowWrap: 'anywhere' }}>
          {item.description}
        </Typography>
      )}
      <Typography variant="caption" color="text.secondary">
        {time}
      </Typography>
      <Stack direction="row" sx={{ gap: 1, flexWrap: 'wrap' }}>
        <Button
          size="small"
          component={RouterLink}
          href={notificationActivity(item)}
          onClick={onClose}
        >
          {t('notification_inbox.view')}
        </Button>
        {!item.is_read && (
          <Button disabled={busy} size="small" onClick={() => change('read')}>
            {t('notification_inbox.read')}
          </Button>
        )}
        {!item.archived && (
          <Button disabled={busy} size="small" onClick={() => change('archive')}>
            {t('notification_inbox.archive')}
          </Button>
        )}
      </Stack>
    </Stack>
  );
}
