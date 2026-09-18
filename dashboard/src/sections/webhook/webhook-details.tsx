import type { WebhookTarget } from 'src/actions/webhooks';
import type { WebhookDelivery, WebhookSubscription } from 'src/lib/swissknife';

import { useState } from 'react';

import {
  Box,
  Card,
  Alert,
  Stack,
  Table,
  Button,
  Dialog,
  MenuItem,
  TableRow,
  Accordion,
  TableBody,
  TableCell,
  TextField,
  Typography,
  DialogTitle,
  DialogActions,
  DialogContent,
  AccordionDetails,
  AccordionSummary,
} from '@mui/material';

import { paths } from 'src/routes/paths';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { fDateTime } from 'src/utils/format-time';
import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';
import { Permission, WebhookDeliveryStatus } from 'src/lib/swissknife';
import {
  useWebhook,
  retryDelivery,
  useWebhookDelivery,
  useWebhookDeliveries,
} from 'src/actions/webhooks';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { CopyButton } from 'src/components/copy';
import { ErrorView } from 'src/components/error';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { TableNoData, TableHeadCustom, TablePaginationCustom } from 'src/components/table';

import { useAuthContext } from 'src/auth/hooks';

import { webhookHref } from './webhook-utils';
import { WebhookActions } from './webhook-actions';
import { DeliveryState } from './webhook-delivery-state';

export function canRetryDelivery(delivery: WebhookDelivery, active: boolean) {
  return (
    active &&
    delivery.status !== WebhookDeliveryStatus.PENDING &&
    !delivery.in_flight &&
    delivery.attempt_count < delivery.max_attempts
  );
}

type Props = {
  isAdmin: boolean;
  id: string;
  walletId?: string;
  canWrite: boolean;
  onBack: VoidFunction;
};

export function WebhookDetails({ isAdmin, id, walletId, canWrite, onBack }: Props) {
  const { t } = useTranslate();
  const { data: subscription, error, isLoading } = useWebhook(isAdmin, id, walletId);
  const router = useRouter();
  const params = useSearchParams();
  const showDelivery = (delivery: string) =>
    router.replace(webhookHref(params, { delivery }), { scroll: false });
  return (
    <Stack spacing={3}>
      <Button
        startIcon={<Iconify icon="eva:arrow-ios-back-fill" />}
        onClick={onBack}
        sx={{ alignSelf: 'flex-start' }}
      >
        {t('webhook.back')}
      </Button>
      {!subscription || error ? (
        <ErrorView errors={[error]} isLoading={[isLoading]} data={[subscription]} />
      ) : (
        <>
          <Card sx={{ p: 3 }}>
            <Stack spacing={2.5}>
              <Stack direction="row" sx={{ alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
                <Typography variant="h6" sx={{ overflowWrap: 'anywhere' }}>
                  {subscription.url}
                </Typography>
                <Label color={subscription.active ? 'success' : 'default'}>
                  {t(subscription.active ? 'webhook.enabled' : 'webhook.disabled')}
                </Label>
              </Stack>
              <Identifier label={t('webhook.webhook_id')} value={subscription.id} />
              <Identifier label={t('webhook.account_id')} value={subscription.account_id} />
              <Identifier label={t('webhook.wallet_id')} value={subscription.wallet_id} />
              <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                {subscription.event_types.map((event) => (
                  <Label key={event} color="info">
                    {event}
                  </Label>
                ))}
              </Stack>
              <Typography variant="caption" color="text.secondary">
                {t('webhook.created')}: {fDateTime(subscription.created_at)}
                {subscription.updated_at
                  ? ` · ${t('webhook.updated_at')}: ${fDateTime(subscription.updated_at)}`
                  : ''}
              </Typography>
              {canWrite && (
                <WebhookActions
                  isAdmin={isAdmin}
                  subscription={subscription}
                  onDeleted={onBack}
                  onQueued={showDelivery}
                />
              )}
            </Stack>
          </Card>
          <DeliveryList isAdmin={isAdmin} subscription={subscription} canWrite={canWrite} />
          <WebhookGuide />
        </>
      )}
    </Stack>
  );
}

function Identifier({ label, value }: { label: string; value: string }) {
  return (
    <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
      <Box sx={{ minWidth: 0, flex: 1 }}>
        <Typography variant="caption" color="text.secondary">
          {label}
        </Typography>
        <Typography variant="body2" sx={{ overflowWrap: 'anywhere' }}>
          {value}
        </Typography>
      </Box>
      <CopyButton value={value} />
    </Stack>
  );
}

function DeliveryList({
  isAdmin,
  subscription,
  canWrite,
}: {
  isAdmin: boolean;
  subscription: WebhookSubscription;
  canWrite: boolean;
}) {
  const { t } = useTranslate();
  const params = useSearchParams();
  const router = useRouter();
  const page = Math.max(0, Number(params.get('delivery_page')) || 0);
  const rows = 10;
  const status = Object.values(WebhookDeliveryStatus).find(
    (value) => value === params.get('delivery_status')
  );
  const selected = params.get('delivery') ?? undefined;
  const { data, error, isLoading } = useWebhookDeliveries(isAdmin, subscription, {
    limit: rows + 1,
    offset: page * rows,
    status,
  });
  const change = (values: Record<string, string | undefined>) =>
    router.replace(webhookHref(params, values), { scroll: false });
  return (
    <>
      <Card>
        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={2}
          sx={{ p: 2.5, justifyContent: 'space-between' }}
        >
          <Stack spacing={0.5}>
            <Typography variant="h6">{t('webhook.deliveries')}</Typography>
            <Typography variant="body2" color="text.secondary">
              {t('webhook.history_help')}
            </Typography>
          </Stack>
          <TextField
            select
            size="small"
            label={t('webhook.delivery_status')}
            value={status ?? ''}
            onChange={(event) =>
              change({ delivery_status: event.target.value, delivery_page: undefined })
            }
            sx={{ minWidth: 150 }}
          >
            <MenuItem value="">{t('all')}</MenuItem>
            {Object.values(WebhookDeliveryStatus).map((value) => (
              <MenuItem key={value} value={value}>
                {t(`webhook.status_${value}`)}
              </MenuItem>
            ))}
          </TextField>
        </Stack>
        {isLoading || error ? (
          <ErrorView errors={[error]} isLoading={[isLoading]} />
        ) : (
          <>
            <Scrollbar>
              <Table sx={{ minWidth: 660 }}>
                <TableHeadCustom
                  headCells={[
                    { id: 'event', label: t('webhook.event') },
                    { id: 'status', label: t('webhook.delivery_status') },
                    { id: 'attempts', label: t('webhook.attempts') },
                    { id: 'response', label: t('webhook.response') },
                    { id: 'created', label: t('webhook.created') },
                    { id: 'details' },
                  ]}
                />
                <TableBody>
                  {data?.slice(0, rows).map((delivery) => (
                    <TableRow key={delivery.id} hover>
                      <TableCell>
                        <Typography variant="subtitle2">{delivery.event_type}</Typography>
                        <Typography variant="caption" color="text.secondary">
                          {delivery.event_id}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <DeliveryState status={delivery.status} />
                      </TableCell>
                      <TableCell>
                        {delivery.attempt_count} / {delivery.max_attempts}
                      </TableCell>
                      <TableCell>{delivery.response_status ?? '—'}</TableCell>
                      <TableCell>{fDateTime(delivery.created_at)}</TableCell>
                      <TableCell>
                        <Button onClick={() => change({ delivery: delivery.id })}>
                          {t('webhook.see_details')}
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                  <TableNoData notFound={!data?.length} />
                </TableBody>
              </Table>
            </Scrollbar>
            <TablePaginationCustom
              count={data && data.length > rows ? -1 : page * rows + (data?.length ?? 0)}
              page={page}
              rowsPerPage={rows}
              rowsPerPageOptions={[rows]}
              onPageChange={(_, value) => change({ delivery_page: String(value) })}
            />
          </>
        )}
      </Card>
      {selected && (
        <DeliveryDetails
          key={selected}
          isAdmin={isAdmin}
          target={subscription}
          deliveryId={selected}
          canWrite={canWrite}
          active={subscription.active}
          onClose={() => change({ delivery: undefined })}
        />
      )}
    </>
  );
}

function DeliveryDetails({
  isAdmin,
  target,
  deliveryId,
  canWrite,
  active,
  onClose,
}: {
  isAdmin: boolean;
  target: WebhookTarget;
  deliveryId: string;
  canWrite: boolean;
  active: boolean;
  onClose: VoidFunction;
}) {
  const { t } = useTranslate();
  const { user } = useAuthContext();
  const { selectWallet } = useAccountContext();
  const router = useRouter();
  const { data, error, isLoading } = useWebhookDelivery(isAdmin, target, deliveryId);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const retry = async () => {
    setBusy(true);
    try {
      await retryDelivery(isAdmin, target, deliveryId);
      setConfirm(false);
      toast.success(t('webhook.queued'));
    } catch (failure) {
      handleActionError(failure);
    } finally {
      setBusy(false);
    }
  };
  const openResource = async () => {
    if (!data?.resource_id) return;
    const invoice = data.event_type.startsWith('invoice.');
    try {
      if (!isAdmin) await selectWallet(target.wallet_id);
      router.push(
        isAdmin
          ? invoice
            ? paths.admin.transactionInvoiceDetail(data.resource_id)
            : paths.admin.transactionPaymentDetail(data.resource_id)
          : invoice
            ? paths.wallet.invoice(data.resource_id)
            : paths.wallet.payment(data.resource_id)
      );
    } catch (failure) {
      handleActionError(failure);
    }
  };
  return (
    <>
      <Dialog open onClose={onClose} maxWidth="md" fullWidth>
        <DialogTitle>{t('webhook.delivery_details')}</DialogTitle>
        <DialogContent>
          {!data || error ? (
            <ErrorView errors={[error]} isLoading={[isLoading]} data={[data]} />
          ) : (
            <Stack spacing={2.5} sx={{ pt: 1 }}>
              <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
                <DeliveryState status={data.status} />
                <Typography variant="body2">
                  {t('webhook.attempt_count', {
                    count: data.attempt_count,
                    max: data.max_attempts,
                  })}
                </Typography>
                {data.in_flight && <Label color="info">{t('webhook.in_flight')}</Label>}
              </Stack>
              <Identifier label={t('webhook.delivery_id')} value={data.id} />
              <Identifier label={t('webhook.event_id')} value={data.event_id} />
              <Typography variant="body2">
                {t('webhook.event')}: {data.event_type}
              </Typography>
              <Typography variant="body2">
                {t('webhook.response')}: {data.response_status ?? '—'}
              </Typography>
              {data.last_error && (
                <Alert severity="warning" sx={{ overflowWrap: 'anywhere' }}>
                  {data.last_error}
                </Alert>
              )}
              <Typography variant="body2">
                {t('webhook.created')}: {fDateTime(data.created_at)}
              </Typography>
              {data.updated_at && (
                <Typography variant="body2">
                  {t('webhook.updated_at')}: {fDateTime(data.updated_at)}
                </Typography>
              )}
              {data.delivered_at && (
                <Typography variant="body2">
                  {t('webhook.delivered_at')}: {fDateTime(data.delivered_at)}
                </Typography>
              )}
              {data.next_attempt_at && (
                <Typography variant="body2">
                  {t('webhook.next_attempt')}: {fDateTime(data.next_attempt_at)}
                </Typography>
              )}
              {data.resource_id &&
                (!isAdmin || user?.permissions.includes(Permission.READ_TRANSACTION)) && (
                  <Button onClick={openResource}>{t('webhook.related_transaction')}</Button>
                )}
              <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                <Typography variant="subtitle2">{t('webhook.payload')}</Typography>
                <CopyButton value={JSON.stringify(data.payload, null, 2)} />
              </Stack>
              <TextField
                multiline
                minRows={6}
                maxRows={16}
                value={JSON.stringify(data.payload, null, 2)}
                slotProps={{
                  input: { readOnly: true, sx: { fontFamily: 'monospace', fontSize: 13 } },
                  htmlInput: { 'aria-label': t('webhook.payload') },
                }}
              />
              {canWrite && (
                <>
                  <Button
                    variant="outlined"
                    disabled={!canRetryDelivery(data, active)}
                    onClick={() => setConfirm(true)}
                  >
                    {t(
                      data.status === WebhookDeliveryStatus.DELIVERED
                        ? 'webhook.redeliver'
                        : 'webhook.retry'
                    )}
                  </Button>
                  <Typography variant="caption" color="text.secondary">
                    {t('webhook.retry_help')}
                  </Typography>
                </>
              )}
            </Stack>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>{t('close')}</Button>
        </DialogActions>
      </Dialog>
      <ConfirmDialog
        open={confirm}
        onClose={() => !busy && setConfirm(false)}
        title={t('webhook.confirm_retry_title')}
        content={t('webhook.confirm_retry')}
        action={
          <Button variant="contained" loading={busy} onClick={retry}>
            {t('confirm')}
          </Button>
        }
      />
    </>
  );
}

const verificationExample = `import { createHmac, timingSafeEqual } from 'node:crypto';

export function verifyWebhook(rawBody, headers, secret) {
  const timestamp = headers['x-swissknife-timestamp'];
  const signature = headers['x-swissknife-signature'];
  if (!/^\\d+$/.test(timestamp ?? '') || typeof signature !== 'string') return false;
  if (Math.abs(Date.now() / 1000 - Number(timestamp)) > 300) return false;
  const digest = createHmac('sha256', Buffer.from(secret, 'base64url'))
    .update(timestamp + '.').update(rawBody).digest('hex');
  const expected = Buffer.from('v1=' + digest);
  const received = Buffer.from(signature);
  return expected.length === received.length && timingSafeEqual(expected, received);
}`;

function WebhookGuide() {
  const { t } = useTranslate();
  return (
    <Accordion>
      <AccordionSummary expandIcon={<Iconify icon="eva:arrow-ios-downward-fill" />}>
        <Typography variant="subtitle1">{t('webhook.integration_guide')}</Typography>
      </AccordionSummary>
      <AccordionDetails>
        <Stack spacing={2}>
          <Typography variant="body2">{t('webhook.verify_help')}</Typography>
          <Box sx={{ position: 'relative' }}>
            <Box sx={{ textAlign: 'right' }}>
              <CopyButton value={verificationExample} />
            </Box>
            <Typography
              component="pre"
              variant="body2"
              sx={{
                fontFamily: 'monospace',
                whiteSpace: 'pre-wrap',
                overflowWrap: 'anywhere',
                p: 2,
                bgcolor: 'background.neutral',
                borderRadius: 1,
              }}
            >
              {verificationExample}
            </Typography>
          </Box>
          <Typography variant="body2">{t('webhook.delivery_contract')}</Typography>
          <Typography variant="body2">{t('webhook.test_contract')}</Typography>
          <Typography variant="body2">{t('webhook.retention_contract')}</Typography>
        </Stack>
      </AccordionDetails>
    </Accordion>
  );
}
