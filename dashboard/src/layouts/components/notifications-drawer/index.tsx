'use client';

import type { Wallet, NotificationAction } from 'src/lib/swissknife';

import { useState } from 'react';
import { useSWRConfig } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Box from '@mui/material/Box';
import Badge from '@mui/material/Badge';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Drawer from '@mui/material/Drawer';
import Button from '@mui/material/Button';
import SvgIcon from '@mui/material/SvgIcon';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';

import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';
import { isNotificationKey, useNotificationPage } from 'src/actions/notifications';
import { updateAccountNotification, markAccountNotificationsRead } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { CustomTabs } from 'src/components/custom-tabs';

import { NotificationItem } from './notification-item';
import { NotificationPreferences } from './notification-preferences';

export function NotificationsDrawer() {
  const { account, wallets } = useAccountContext();
  return account ? <Inbox key={account.id} accountId={account.id} wallets={wallets} /> : null;
}

function Inbox({ accountId, wallets }: { accountId: string; wallets: Wallet[] }) {
  const { t } = useTranslate();
  const { mutate } = useSWRConfig();
  const drawer = useBoolean();
  const [tab, setTab] = useState<'inbox' | 'archived' | 'preferences'>('inbox');
  const archived = tab === 'archived';
  const [cursors, setCursors] = useState<number[]>([]);
  const [busy, setBusy] = useState(false);
  const first = useNotificationPage(accountId);
  const page = useNotificationPage(
    drawer.value && tab !== 'preferences' ? accountId : undefined,
    archived,
    cursors.at(-1)
  );
  const unread = first.data?.unread_count ?? 0;
  const newest = first.data?.items[0]?.source_event_id;
  const nextBefore = page.data?.next_before;

  async function change(id: number, action: NotificationAction | 'all') {
    setBusy(true);
    try {
      const result =
        action === 'all'
          ? await markAccountNotificationsRead({ body: { through: id } })
          : await updateAccountNotification({ path: { id }, body: { action } });
      if (result.error) throw new Error(result.error.reason);
      await mutate((key) => isNotificationKey(key, accountId));
    } catch {
      toast.error(t('notification_inbox.update_error'));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Tooltip title={t('notification_inbox.title')}>
        <IconButton
          aria-label={t('notification_inbox.open', { count: unread })}
          aria-expanded={drawer.value}
          onClick={drawer.onTrue}
        >
          <Badge badgeContent={unread} color="error">
            <SvgIcon>
              {/* https://icon-sets.iconify.design/solar/bell-bing-bold-duotone/ */}
              <path
                fill="currentColor"
                d="M18.75 9v.704c0 .845.24 1.671.692 2.374l1.108 1.723c1.011 1.574.239 3.713-1.52 4.21a25.794 25.794 0 0 1-14.06 0c-1.759-.497-2.531-2.636-1.52-4.21l1.108-1.723a4.393 4.393 0 0 0 .693-2.374V9c0-3.866 3.022-7 6.749-7s6.75 3.134 6.75 7"
                opacity="0.5"
              />
              <path
                fill="currentColor"
                d="M12.75 6a.75.75 0 0 0-1.5 0v4a.75.75 0 0 0 1.5 0zM7.243 18.545a5.002 5.002 0 0 0 9.513 0c-3.145.59-6.367.59-9.513 0"
              />
            </SvgIcon>
          </Badge>
        </IconButton>
      </Tooltip>
      <Drawer
        open={drawer.value}
        onClose={drawer.onFalse}
        anchor="right"
        slotProps={{ paper: { sx: { width: 1, maxWidth: 420 } } }}
      >
        <Stack direction="row" sx={{ alignItems: 'center', px: 2.5, py: 2, gap: 1 }}>
          <Typography variant="h6" sx={{ flexGrow: 1 }}>
            {t('notification_inbox.title')}
          </Typography>
          <IconButton aria-label={t('close')} onClick={drawer.onFalse}>
            <Iconify icon="mingcute:close-line" />
          </IconButton>
        </Stack>
        <CustomTabs
          value={tab}
          onChange={(_, selected) => {
            setTab(selected);
            setCursors([]);
          }}
          sx={{ px: 2.5 }}
        >
          <Tab value="inbox" label={t('notification_inbox.inbox')} />
          <Tab value="archived" label={t('notification_inbox.archived')} />
          <Tab value="preferences" label={t('notification_inbox.preferences')} />
        </CustomTabs>
        {tab === 'preferences' ? (
          <NotificationPreferences />
        ) : (
          <>
            {!archived && unread > 0 && newest != null && (
              <Button
                size="small"
                disabled={busy}
                onClick={() => change(newest, 'all')}
                sx={{ alignSelf: 'flex-end', mr: 2.5, mt: 1 }}
              >
                {t('notification_inbox.all_read')}
              </Button>
            )}
            <Scrollbar>
              {page.error && (
                <Alert
                  severity="error"
                  action={
                    <Button onClick={() => page.mutate()}>{t('notification_inbox.retry')}</Button>
                  }
                >
                  {t('notification_inbox.load_error')}
                </Alert>
              )}
              {page.isLoading && (
                <Box sx={{ p: 3 }}>
                  <CircularProgress size={24} aria-label={t('notification_inbox.loading')} />
                </Box>
              )}
              {page.data && !page.data.items.length && (
                <Typography color="text.secondary" sx={{ p: 2.5 }}>
                  {t(archived ? 'notification_inbox.empty_archive' : 'notification_inbox.empty')}
                </Typography>
              )}
              <Box component="ul" sx={{ p: 0, m: 0, listStyle: 'none' }}>
                {page.data?.items.map((item) => (
                  <NotificationItem
                    key={item.source_event_id}
                    item={item}
                    wallet={wallets.find((w) => w.id === item.wallet_id)?.label || item.wallet_id}
                    busy={busy}
                    change={(action) => change(item.source_event_id, action)}
                    onClose={drawer.onFalse}
                  />
                ))}
              </Box>
            </Scrollbar>
            <Stack direction="row" sx={{ justifyContent: 'space-between', px: 2.5, py: 1 }}>
              {cursors.length > 0 && (
                <Button onClick={() => setCursors((c) => c.slice(0, -1))}>
                  {t('notification_inbox.newer')}
                </Button>
              )}
              {nextBefore != null && (
                <Button onClick={() => setCursors((c) => [...c, nextBefore])}>
                  {t('notification_inbox.older')}
                </Button>
              )}
            </Stack>
          </>
        )}
      </Drawer>
    </>
  );
}
