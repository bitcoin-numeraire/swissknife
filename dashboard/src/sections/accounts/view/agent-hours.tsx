'use client';

import { useState } from 'react';

import {
  Stack,
  Alert,
  Button,
  Dialog,
  Divider,
  TextField,
  Typography,
  DialogTitle,
  ToggleButton,
  DialogContent,
  DialogActions,
  LinearProgress,
  ToggleButtonGroup,
} from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useAgentHours } from 'src/actions/spending-policy';
import { Permission, setAgentSpendingHours, clearAgentSpendingHours } from 'src/lib/swissknife';

import { ConfirmDialog } from 'src/components/custom-dialog';

import { RoleBasedGuard } from 'src/auth/guard';

type Form = { weekdays: number[]; start: string; end: string };
const days = [1, 2, 3, 4, 5, 6, 7];
const clock = (minute: number) =>
  [Math.floor((minute % 1440) / 60), minute % 60].map((v) => String(v).padStart(2, '0')).join(':');
function minutes(value: string) {
  if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(value)) return null;
  const [hour, minute] = value.split(':').map(Number);
  return hour * 60 + minute;
}

export function AgentHours({ accountId }: { accountId: string }) {
  const { t, currentLang } = useTranslate();
  const { data, error, isLoading, mutate } = useAgentHours(accountId);
  const [form, setForm] = useState<Form | null>(null);
  const [remove, setRemove] = useState(false);
  const [saving, setSaving] = useState(false);
  const start = minutes(form?.start ?? '');
  const end = minutes(form?.end ?? '');
  const valid =
    form &&
    form.weekdays.length > 0 &&
    start !== null &&
    end !== null &&
    (start !== end || end === 0);
  const dayName = (day: number, weekday: 'short' | 'long') =>
    new Intl.DateTimeFormat(currentLang.value, { weekday, timeZone: 'UTC' }).format(
      new Date(Date.UTC(2024, 0, day))
    );
  function edit() {
    setForm(
      data
        ? { weekdays: data.weekdays, start: clock(data.start_minute), end: clock(data.end_minute) }
        : { weekdays: [1, 2, 3, 4, 5], start: '09:00', end: '17:00' }
    );
  }
  async function submit(clear: boolean) {
    if (!clear && !valid) return;
    setSaving(true);
    try {
      const path = { account_id: accountId };
      if (clear) await clearAgentSpendingHours<true>({ path });
      else if (form && start !== null && end !== null)
        await setAgentSpendingHours<true>({
          path,
          body: {
            weekdays: form.weekdays,
            start_minute: start,
            end_minute: end === 0 ? 1440 : end,
          },
        });
      await mutate();
      setForm(null);
      setRemove(false);
    } catch (cause) {
      handleActionError(cause);
    } finally {
      setSaving(false);
    }
  }
  return (
    <Stack spacing={1.5}>
      <Divider />
      <Typography component="h3" variant="subtitle1">
        {t('agent_hours.title')}
      </Typography>
      {isLoading ? (
        <LinearProgress />
      ) : error ? (
        <Alert severity="error">{t('agent_hours.error')}</Alert>
      ) : (
        <Typography variant="body2">
          {data ? (
            <>
              {data.weekdays.map((day) => dayName(day, 'short')).join(', ')} ·{' '}
              {clock(data.start_minute)}–{clock(data.end_minute)} UTC{' '}
              {data.end_minute <= data.start_minute || data.end_minute === 1440
                ? '(' + t('agent_hours.next_day') + ')'
                : ''}
            </>
          ) : (
            t('agent_hours.none')
          )}
        </Typography>
      )}
      {!isLoading && !error && (
        <RoleBasedGuard permissions={[Permission.WRITE_ACCOUNT]}>
          <Stack direction="row" spacing={1}>
            <Button variant="outlined" onClick={edit}>
              {t(data ? 'agent_hours.edit' : 'agent_hours.set')}
            </Button>
            {data && (
              <Button color="error" onClick={() => setRemove(true)}>
                {t('agent_hours.remove')}
              </Button>
            )}
          </Stack>
        </RoleBasedGuard>
      )}
      <Dialog open={!!form} onClose={() => !saving && setForm(null)} fullWidth maxWidth="xs">
        <DialogTitle>{t('agent_hours.edit')}</DialogTitle>
        <DialogContent>
          {form && (
            <Stack spacing={2} sx={{ pt: 1 }}>
              <Typography variant="body2" color="text.secondary">
                {t('agent_hours.help')}
              </Typography>
              <ToggleButtonGroup
                size="small"
                value={form.weekdays}
                disabled={saving}
                aria-label={t('agent_hours.days')}
                onChange={(_, selected: number[]) => setForm({ ...form, weekdays: selected })}
                sx={{ flexWrap: 'wrap' }}
              >
                {days.map((day) => (
                  <ToggleButton key={day} value={day} aria-label={dayName(day, 'long')}>
                    {dayName(day, 'short')}
                  </ToggleButton>
                ))}
              </ToggleButtonGroup>
              <TextField
                label={t('agent_hours.start')}
                type="time"
                value={form.start}
                disabled={saving}
                onChange={(e) => setForm({ ...form, start: e.target.value })}
                slotProps={{ inputLabel: { shrink: true } }}
              />
              <TextField
                label={t('agent_hours.end')}
                type="time"
                value={form.end}
                disabled={saving}
                onChange={(e) => setForm({ ...form, end: e.target.value })}
                slotProps={{ inputLabel: { shrink: true } }}
              />
              {!valid && <Alert severity="info">{t('agent_hours.invalid')}</Alert>}
            </Stack>
          )}
        </DialogContent>
        <DialogActions>
          <Button disabled={saving} onClick={() => setForm(null)}>
            {t('spending_policy.cancel')}
          </Button>
          <Button variant="contained" disabled={saving || !valid} onClick={() => submit(false)}>
            {t('agent_hours.save')}
          </Button>
        </DialogActions>
      </Dialog>
      <ConfirmDialog
        open={remove}
        onClose={() => !saving && setRemove(false)}
        title={t('agent_hours.remove')}
        content={t('agent_hours.confirm')}
        action={
          <Button variant="contained" color="error" disabled={saving} onClick={() => submit(true)}>
            {t('agent_hours.remove')}
          </Button>
        }
      />
    </Stack>
  );
}
