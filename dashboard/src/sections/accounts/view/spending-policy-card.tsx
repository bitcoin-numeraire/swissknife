'use client';

import { useState } from 'react';

import {
  Box,
  Card,
  Stack,
  Alert,
  Button,
  Dialog,
  TextField,
  CardHeader,
  Typography,
  CardContent,
  DialogTitle,
  DialogActions,
  DialogContent,
  LinearProgress,
} from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useSpendingPolicy } from 'src/actions/spending-policy';
import {
  Permission,
  disableAccountSpendingPolicy,
  replaceAccountSpendingPolicy,
} from 'src/lib/swissknife';

import { ConfirmDialog } from 'src/components/custom-dialog';

import { RoleBasedGuard } from 'src/auth/guard';

import {
  periods,
  editLimits,
  limitFields,
  parseLimits,
  usedPercent,
  satsFromMsat,
  type LimitsForm,
} from './spending-limits';

export function SpendingPolicyCard({ accountId }: { accountId: string }) {
  const { t, currentLang } = useTranslate();
  const { data, error, isLoading, mutate } = useSpendingPolicy(accountId);
  const [form, setForm] = useState<LimitsForm | null>(null);
  const [confirmDisable, setConfirmDisable] = useState(false);
  const [pending, setPending] = useState(false);
  const parsed = form && parseLimits(form);
  const path = { account_id: accountId };
  const number = Intl.NumberFormat(currentLang.value, { maximumFractionDigits: 3 });
  const sats = (value: string) => number.format(satsFromMsat(value) as `${number}`);

  async function save(disable: boolean) {
    if (!disable && !parsed) return;
    setPending(true);
    try {
      if (disable) await disableAccountSpendingPolicy<true>({ path });
      else await replaceAccountSpendingPolicy<true>({ path, body: parsed! });
      await mutate();
      setForm(null);
      setConfirmDisable(false);
    } catch (cause) {
      handleActionError(cause);
    } finally {
      setPending(false);
    }
  }

  return (
    <Card>
      <CardHeader
        disableTypography
        title={
          <Typography component="h2" variant="h6">
            {t('spending_policy.title')}
          </Typography>
        }
      />
      <CardContent>
        <Stack spacing={2.5}>
          {error ? (
            <Alert severity="error">{t('spending_policy.load_error')}</Alert>
          ) : isLoading ? (
            <Typography color="text.secondary">{t('spending_policy.loading')}</Typography>
          ) : data ? (
            <>
              <Typography variant="body2" color="text.secondary">
                {t('spending_policy.periods')}
              </Typography>
              {(['per_payment_msat', 'agent_hourly_msat'] as const).map((field) => (
                <Typography key={field} variant="body2">
                  {t(`spending_policy.${field}`)} ·{' '}
                  {data.limits[field] == null
                    ? t('spending_policy.unlimited')
                    : t('spending_policy.sats', { value: sats(data.limits[field]) })}
                </Typography>
              ))}
              {periods.map((field) => {
                const cap = data.limits[field];
                const used = data.usage[field];
                return (
                  <Box key={field}>
                    <Stack direction="row" sx={{ justifyContent: 'space-between' }} spacing={1}>
                      <Typography variant="subtitle2">{t(`spending_policy.${field}`)}</Typography>
                      <Typography variant="body2">
                        {cap == null
                          ? t('spending_policy.uncapped_usage', { used: sats(used) })
                          : t('spending_policy.usage', { used: sats(used), cap: sats(cap) })}
                      </Typography>
                    </Stack>
                    {cap != null && (
                      <LinearProgress
                        aria-label={t(`spending_policy.${field}`)}
                        variant="determinate"
                        value={usedPercent(used, cap)}
                        sx={{ mt: 1 }}
                      />
                    )}
                  </Box>
                );
              })}
            </>
          ) : (
            <Typography color="text.secondary">{t('spending_policy.inactive')}</Typography>
          )}
          {!error && !isLoading && (
            <RoleBasedGuard permissions={[Permission.WRITE_ACCOUNT]}>
              <Stack direction="row" spacing={1}>
                <Button variant="outlined" onClick={() => setForm(editLimits(data?.limits))}>
                  {t(data ? 'spending_policy.edit' : 'spending_policy.create')}
                </Button>
                {data && (
                  <Button color="error" onClick={() => setConfirmDisable(true)}>
                    {t('spending_policy.disable')}
                  </Button>
                )}
              </Stack>
            </RoleBasedGuard>
          )}
        </Stack>
      </CardContent>
      <Dialog open={!!form} onClose={() => !pending && setForm(null)} fullWidth maxWidth="xs">
        <DialogTitle>{t('spending_policy.edit')}</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ pt: 1 }}>
            <Typography variant="body2" color="text.secondary">
              {t('spending_policy.input_help')}
            </Typography>
            {form &&
              limitFields.map((field) => (
                <TextField
                  key={field}
                  label={t(`spending_policy.${field}`)}
                  value={form[field]}
                  disabled={pending}
                  slotProps={{ htmlInput: { inputMode: 'decimal', maxLength: 20 } }}
                  onChange={(event) => setForm({ ...form, [field]: event.target.value })}
                />
              ))}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button disabled={pending} onClick={() => setForm(null)}>
            {t('spending_policy.cancel')}
          </Button>
          <Button variant="contained" disabled={!parsed || pending} onClick={() => save(false)}>
            {t('spending_policy.save')}
          </Button>
        </DialogActions>
      </Dialog>
      <ConfirmDialog
        open={confirmDisable}
        onClose={() => !pending && setConfirmDisable(false)}
        title={t('spending_policy.disable')}
        content={t('spending_policy.disable_help')}
        action={
          <Button variant="contained" color="error" disabled={pending} onClick={() => save(true)}>
            {t('spending_policy.disable')}
          </Button>
        }
      />
    </Card>
  );
}
