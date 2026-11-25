'use client';

import type { PaymentApproval, ApprovalDecision } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useState } from 'react';

import {
  Box,
  Card,
  List,
  Alert,
  Stack,
  Button,
  Dialog,
  ListItem,
  CardHeader,
  Typography,
  CardContent,
  DialogTitle,
  ListItemText,
  DialogContent,
  DialogActions,
  LinearProgress,
} from '@mui/material';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { useSpendingPolicy } from 'src/actions/spending-policy';
import { Permission, listPaymentApprovals, decidePaymentApproval } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

import { RoleBasedGuard } from 'src/auth/guard';

export function PaymentApprovalCard({ accountId }: { accountId: string }) {
  const { t, i18n } = useTranslate();
  const policy = useSpendingPolicy(accountId);
  const enabled = policy.data?.limits.approval_above_msat != null;
  const [selected, setSelected] = useState<PaymentApproval | null>(null);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);
  const queue = useSWR(
    endpointKeys.accounts.paymentApprovals(accountId),
    async () =>
      (await listPaymentApprovals<true>({ path: { account_id: accountId }, throwOnError: true }))
        .data,
    { refreshInterval: enabled ? 30000 : 0 }
  );
  const number = new Intl.NumberFormat(i18n.language);
  const amount = (text: string) =>
    t('payment_review.amount', { value: number.format(BigInt(text)) });
  const time = (date: Date) =>
    new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium', timeStyle: 'short' }).format(
      date
    );
  async function decide(decision: ApprovalDecision) {
    if (!selected || saving) return;
    setSaving(true);
    setFailed(false);
    try {
      await decidePaymentApproval<true>({
        path: { account_id: accountId, id: selected.id },
        body: { decision },
        throwOnError: true,
      });
      await queue.mutate((current) => current?.filter((item) => item.id !== selected.id), {
        revalidate: false,
      });
      setSelected(null);
      toast.success(
        t(decision === 'approve' ? 'payment_review.approved' : 'payment_review.rejected')
      );
    } catch {
      setFailed(true);
      void queue.mutate().catch(() => {});
    } finally {
      setSaving(false);
    }
  }
  if (!enabled && !queue.error && !queue.data?.length) return null;

  return (
    <Card>
      <CardHeader title={t('payment_review.title')} />
      <CardContent>
        {queue.error && <Alert severity="error">{t('payment_review.load_error')}</Alert>}
        {queue.isLoading && <LinearProgress aria-label={t('payment_review.title')} />}
        {queue.data &&
          !queue.error &&
          (queue.data.length ? (
            <List disablePadding>
              {queue.data.map((item) => (
                <ListItem key={item.id} disableGutters divider>
                  <ListItemText
                    primary={amount(item.amount_msat)}
                    secondary={item.destination}
                    sx={{
                      minWidth: 0,
                      mr: 2,
                      '& .MuiListItemText-secondary': { overflowWrap: 'anywhere' },
                    }}
                  />
                  <Button
                    variant="outlined"
                    onClick={() => {
                      setFailed(false);
                      setSelected(item);
                    }}
                  >
                    {t('payment_review.review')}
                  </Button>
                </ListItem>
              ))}
            </List>
          ) : (
            <Typography color="text.secondary">{t('payment_review.empty')}</Typography>
          ))}
      </CardContent>
      {selected && (
        <Dialog
          open
          fullWidth
          maxWidth="sm"
          onClose={() => !saving && setSelected(null)}
          aria-labelledby="approval-review-title"
        >
          <DialogTitle id="approval-review-title">{t('payment_review.review')}</DialogTitle>
          <DialogContent>
            <Stack component="dl" spacing={1.5} sx={{ m: 0, mb: 2 }}>
              {(
                [
                  ['principal', amount(selected.amount_msat)],
                  ['fee', amount(selected.maximum_fee_msat)],
                  [
                    'maximum',
                    amount(
                      (BigInt(selected.amount_msat) + BigInt(selected.maximum_fee_msat)).toString()
                    ),
                  ],
                  ['input', selected.requested_input],
                  ['destination', selected.destination],
                  ['wallet', selected.wallet_id],
                  ['credential', selected.agent_key_id],
                  ['expires', time(selected.expires_at)],
                ] as const
              ).map(([field, value]) => (
                <Box key={field}>
                  <Typography component="dt" variant="caption" color="text.secondary">
                    {t('payment_review.' + field)}
                  </Typography>
                  <Typography
                    component="dd"
                    variant="body2"
                    sx={{ m: 0, overflowWrap: 'anywhere' }}
                  >
                    {value}
                  </Typography>
                </Box>
              ))}
            </Stack>
            <Typography variant="body2" color="text.secondary">
              {t('payment_review.retry_help')}
            </Typography>
            {failed && (
              <Alert severity="error" sx={{ mt: 2 }}>
                {t('payment_review.decision_error')}
              </Alert>
            )}
          </DialogContent>
          <DialogActions sx={{ flexWrap: 'wrap' }}>
            <Button disabled={saving} onClick={() => setSelected(null)}>
              {t('close')}
            </Button>
            <RoleBasedGuard permissions={[Permission.WRITE_ACCOUNT]}>
              <Button disabled={saving} color="error" onClick={() => decide('reject')}>
                {t('payment_review.reject')}
              </Button>
              <Button disabled={saving} variant="contained" onClick={() => decide('approve')}>
                {t('payment_review.approve')}
              </Button>
            </RoleBasedGuard>
          </DialogActions>
        </Dialog>
      )}
    </Card>
  );
}
