import type { NotificationPreference } from 'src/contexts/account/notification-preferences';

import { useState } from 'react';

import Stack from '@mui/material/Stack';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';

import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';
import { patchAccountPreferences } from 'src/lib/swissknife';
import { notificationPreferences } from 'src/contexts/account/notification-preferences';

import { toast } from 'src/components/snackbar';

export function NotificationPreferences() {
  const { t } = useTranslate();
  const { account, refreshAccount } = useAccountContext();
  const [pending, setPending] = useState<{ key: NotificationPreference; value: boolean }>();
  const preferences = notificationPreferences(account?.preferences?.dashboard_settings);
  const keys = ['incoming', 'outgoing', 'sound'] as const;

  async function save(key: NotificationPreference, value: boolean) {
    setPending({ key, value });
    try {
      await patchAccountPreferences<true>({
        body: { dashboard_settings: { notifications: { [key]: value } } },
      });
      await refreshAccount();
    } catch {
      toast.error(t('notification_inbox.update_error'));
    } finally {
      setPending(undefined);
    }
  }

  return (
    <Stack sx={{ p: 2.5 }} spacing={1}>
      {keys.map((key) => (
        <FormControlLabel
          key={key}
          label={t(
            key === 'sound' ? 'notification_inbox.sound' : `notification_inbox.${key}_alerts`
          )}
          control={
            <Switch
              checked={pending?.key === key ? pending.value : preferences[key]}
              disabled={Boolean(pending)}
              onChange={(_, checked) => save(key, checked)}
            />
          }
        />
      ))}
    </Stack>
  );
}
