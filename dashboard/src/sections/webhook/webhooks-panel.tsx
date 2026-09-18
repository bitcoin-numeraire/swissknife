'use client';

import type { WebhookSubscription } from 'src/lib/swissknife';

import { useState } from 'react';

import {
  Card,
  Link,
  Alert,
  Stack,
  Table,
  Button,
  Tooltip,
  MenuItem,
  TableRow,
  TableBody,
  TableCell,
  TextField,
  IconButton,
  Typography,
} from '@mui/material';

import { RouterLink } from 'src/routes/components';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { fDateTime } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';
import { OrderDirection } from 'src/lib/swissknife';
import { useAccountContext } from 'src/contexts/account';
import { useWebhooks, useWebhookDeliveries } from 'src/actions/webhooks';

import { Label } from 'src/components/label';
import { ErrorView } from 'src/components/error';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { TableNoData, TableHeadCustom, TablePaginationCustom } from 'src/components/table';

import { webhookHref } from './webhook-utils';
import { WebhookActions } from './webhook-actions';
import { WebhookDetails } from './webhook-details';
import { DeliveryState } from './webhook-delivery-state';
import { WebhookForm, walletLabel } from './webhook-form';

function LatestDelivery({ isAdmin, row }: { isAdmin: boolean; row: WebhookSubscription }) {
  const { t } = useTranslate();
  const { data, error, isLoading } = useWebhookDeliveries(isAdmin, row, { limit: 1 }, 30000);
  if (error)
    return (
      <Typography variant="caption" color="error">
        {t('webhook.delivery_load_error')}
      </Typography>
    );
  return data?.[0] ? (
    <DeliveryState status={data[0].status} />
  ) : (
    <Typography variant="caption" color="text.secondary">
      {t(isLoading ? 'loading' : 'webhook.no_deliveries')}
    </Typography>
  );
}

type Props = { isAdmin: boolean; canRead: boolean; canWrite: boolean };

export function WebhooksPanel({ isAdmin, canRead, canWrite }: Props) {
  const { t } = useTranslate();
  const router = useRouter();
  const params = useSearchParams();
  const [creating, setCreating] = useState(false);
  const [knownId, setKnownId] = useState(params.get('id') ?? '');
  const id = params.get('id') ?? undefined;
  const walletId = params.get('wallet_id') ?? undefined;
  const closeDetails = () =>
    router.push(webhookHref(params, { id: undefined, wallet_id: undefined, delivery: undefined }));
  const openDetails = (targetId: string, targetWallet: string) =>
    router.push(
      webhookHref(params, { id: targetId, wallet_id: targetWallet, delivery: undefined })
    );
  return (
    <Stack spacing={3}>
      <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', gap: 2 }}>
        <Typography variant="h5">
          {t(isAdmin ? 'webhook.instance_title' : 'webhook.my_title')}
        </Typography>
        {canWrite && (
          <Button
            variant="contained"
            startIcon={<Iconify icon="mingcute:add-line" />}
            onClick={() => setCreating(true)}
          >
            {t('webhook.create')}
          </Button>
        )}
      </Stack>
      {!canRead ? (
        <Card sx={{ p: 3 }}>
          <Stack spacing={3}>
            <Alert severity="info">{t('webhook.write_only')}</Alert>
            <TextField
              label={t('webhook.webhook_id')}
              value={knownId}
              onChange={(event) => setKnownId(event.target.value)}
            />
            {/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(knownId) && (
              <WebhookActions
                key={knownId}
                isAdmin
                id={knownId}
                onDeleted={() => {
                  setKnownId('');
                  closeDetails();
                }}
              />
            )}
          </Stack>
        </Card>
      ) : id ? (
        <WebhookDetails
          key={`${id}-${isAdmin}`}
          isAdmin={isAdmin}
          id={id}
          walletId={walletId}
          canWrite={canWrite}
          onBack={closeDetails}
        />
      ) : (
        <WebhookList isAdmin={isAdmin} />
      )}
      {creating && (
        <WebhookForm
          isAdmin={isAdmin}
          onClose={() => setCreating(false)}
          onCreated={(targetId, targetWallet) => {
            setKnownId(targetId);
            openDetails(targetId, targetWallet);
          }}
        />
      )}
    </Stack>
  );
}

function WebhookList({ isAdmin }: { isAdmin: boolean }) {
  const { t } = useTranslate();
  const params = useSearchParams();
  const router = useRouter();
  const { wallets } = useAccountContext();
  const [accountFilter, setAccountFilter] = useState(params.get('filter_account_id') ?? '');
  const [walletFilter, setWalletFilter] = useState(params.get('filter_wallet_id') ?? '');
  const page = Math.max(0, Number(params.get('page')) || 0);
  const rows = 10;
  const active = params.get('active') ?? '';
  const order = params.get('order') === 'oldest' ? OrderDirection.ASC : OrderDirection.DESC;
  const { data, error, isLoading, mutate } = useWebhooks(isAdmin, {
    limit: rows + 1,
    offset: page * rows,
    order_direction: order,
    account_id: isAdmin ? params.get('filter_account_id') || undefined : undefined,
    wallet_id: params.get('filter_wallet_id') || undefined,
    active: active === '' ? undefined : active === 'true',
  });
  const change = (values: Record<string, string | undefined>) =>
    router.replace(webhookHref(params, { page: undefined, ...values }), { scroll: false });
  const invalid = [accountFilter, walletFilter].some(
    (value) => value && !/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(value)
  );
  return (
    <Card>
      <Stack spacing={2} sx={{ p: 2.5 }}>
        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
          {isAdmin && (
            <TextField
              fullWidth
              size="small"
              label={t('webhook.account_id')}
              value={accountFilter}
              onChange={(event) => setAccountFilter(event.target.value)}
            />
          )}
          {isAdmin ? (
            <TextField
              fullWidth
              size="small"
              label={t('webhook.wallet_id')}
              value={walletFilter}
              onChange={(event) => setWalletFilter(event.target.value)}
            />
          ) : (
            <TextField
              fullWidth
              select
              size="small"
              label={t('webhook.wallet')}
              value={params.get('filter_wallet_id') ?? ''}
              onChange={(event) => change({ filter_wallet_id: event.target.value })}
            >
              <MenuItem value="">{t('webhook.all_wallets')}</MenuItem>
              {wallets.map((wallet) => (
                <MenuItem key={wallet.id} value={wallet.id}>
                  {walletLabel(wallet)}
                </MenuItem>
              ))}
            </TextField>
          )}
          {isAdmin && (
            <Button
              disabled={invalid}
              onClick={() =>
                change({ filter_account_id: accountFilter, filter_wallet_id: walletFilter })
              }
            >
              {t('webhook.apply_filters')}
            </Button>
          )}
        </Stack>
        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
          <TextField
            fullWidth
            select
            size="small"
            label={t('webhook.enabled_state')}
            value={active}
            onChange={(event) => change({ active: event.target.value })}
          >
            <MenuItem value="">{t('all')}</MenuItem>
            <MenuItem value="true">{t('webhook.enabled')}</MenuItem>
            <MenuItem value="false">{t('webhook.disabled')}</MenuItem>
          </TextField>
          <TextField
            fullWidth
            select
            size="small"
            label={t('webhook.order')}
            value={order === OrderDirection.ASC ? 'oldest' : 'newest'}
            onChange={(event) => change({ order: event.target.value })}
          >
            <MenuItem value="newest">{t('webhook.newest')}</MenuItem>
            <MenuItem value="oldest">{t('webhook.oldest')}</MenuItem>
          </TextField>
          <Tooltip title={t('refresh')}>
            <IconButton
              onClick={() => mutate()}
              aria-label={t('refresh')}
              sx={{ alignSelf: 'flex-end' }}
            >
              <Iconify icon="solar:restart-bold" />
            </IconButton>
          </Tooltip>
        </Stack>
      </Stack>
      {isLoading || error ? (
        <ErrorView errors={[error]} isLoading={[isLoading]} />
      ) : (
        <>
          <Scrollbar>
            <Table sx={{ minWidth: 720 }}>
              <TableHeadCustom
                headCells={[
                  { id: 'url', label: t('webhook.destination') },
                  { id: 'wallet', label: t('webhook.wallet') },
                  { id: 'events', label: t('webhook.events') },
                  { id: 'state', label: t('webhook.enabled_state') },
                  { id: 'delivery', label: t('webhook.latest_delivery') },
                ]}
              />
              <TableBody>
                {data?.slice(0, rows).map((row) => (
                  <TableRow key={row.id} hover>
                    <TableCell sx={{ maxWidth: 260 }}>
                      <Stack spacing={0.5}>
                        <Link
                          component={RouterLink}
                          href={webhookHref(params, { id: row.id, wallet_id: row.wallet_id })}
                          sx={{ overflowWrap: 'anywhere' }}
                        >
                          {row.url}
                        </Link>
                        <Typography variant="caption" color="text.secondary">
                          {fDateTime(row.created_at)}
                        </Typography>
                        {row.updated_at && (
                          <Typography variant="caption" color="text.secondary">
                            {t('webhook.updated_at')}: {fDateTime(row.updated_at)}
                          </Typography>
                        )}
                        {isAdmin && (
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{ overflowWrap: 'anywhere' }}
                          >
                            {t('webhook.account_id')}: {row.account_id}
                          </Typography>
                        )}
                      </Stack>
                    </TableCell>
                    <TableCell sx={{ maxWidth: 180, overflowWrap: 'anywhere' }}>
                      {wallets.find((wallet) => wallet.id === row.wallet_id)?.label ||
                        row.wallet_id}
                    </TableCell>
                    <TableCell>
                      <Stack spacing={0.5}>
                        {row.event_types.map((event) => (
                          <Typography key={event} variant="caption">
                            {event}
                          </Typography>
                        ))}
                      </Stack>
                    </TableCell>
                    <TableCell>
                      <Label color={row.active ? 'success' : 'default'}>
                        {t(row.active ? 'webhook.enabled' : 'webhook.disabled')}
                      </Label>
                    </TableCell>
                    <TableCell>
                      <LatestDelivery isAdmin={isAdmin} row={row} />
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
            onPageChange={(_, value) => change({ page: String(value) })}
          />
        </>
      )}
    </Card>
  );
}
