'use client';

import type { LabelColor } from 'src/components/label';
import type { ITransaction } from 'src/types/transaction';
import type { Invoice, Payment } from 'src/lib/swissknife';

import { useBoolean, useCopyToClipboard } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import ButtonBase from '@mui/material/ButtonBase';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';

import { fDateTime } from 'src/utils/format-time';
import { getLedgerLabel } from 'src/utils/transactions';
import { compactBitcoinAddress } from 'src/utils/bitcoin-request';
import { txidFromOutpoint, bitcoinTransactionExplorerUrl } from 'src/utils/bitcoin-explorer';

import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ConfirmDialog } from 'src/components/custom-dialog';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

const drawerSx = {
  width: { xs: 1, sm: 520 },
  maxWidth: 1,
};

type TransactionQuickDrawerProps = {
  row: ITransaction | null;
  title?: string;
  detailHref?: string;
  canDelete?: boolean;
  onDeleteRow?: () => Promise<void>;
  onClose: VoidFunction;
};

function txAmount(tx: ITransaction) {
  return (tx.amount_msat || 0) + (tx.fee_msat || 0);
}

function isOpenAmountRequest(tx: ITransaction) {
  return (
    tx.transaction_type === TransactionType.INVOICE && tx.status === 'Pending' && !tx.amount_msat
  );
}

function txDirection(tx: ITransaction) {
  return tx.transaction_type === TransactionType.INVOICE ? 'in' : 'out';
}

function statusColor(status: string): LabelColor {
  if (status === 'Settled') return 'success';
  if (status === 'Failed' || status === 'Expired') return 'error';
  return 'warning';
}

function txDisplayTitle(tx: ITransaction, t: (key: string) => string) {
  if (tx.description) return tx.description;

  if (tx.transaction_type === TransactionType.INVOICE) {
    if (isOpenAmountRequest(tx)) return t('wallet_view.open_amount_request');
    if (tx.status === 'Pending') return t('wallet_view.payment_request');
    if (tx.status === 'Expired') return t('wallet_view.request_expired');
    return t('wallet_view.payment_received');
  }

  if (tx.status === 'Pending') return t('wallet_view.payment_pending');
  if (tx.status === 'Failed') return t('wallet_view.payment_failed');
  return t('wallet_view.payment_sent');
}

function compactIdentifier(value: string) {
  if (value.length <= 18) return value;

  return `${value.slice(0, 8)}...${value.slice(-6)}`;
}

export function TransactionQuickDrawer({
  row,
  title,
  detailHref,
  canDelete = false,
  onDeleteRow,
  onClose,
}: TransactionQuickDrawerProps) {
  const { t } = useTranslate();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  if (!row) {
    return (
      <Drawer
        anchor="right"
        open={false}
        onClose={onClose}
        transitionDuration={{ enter: 225, exit: 195 }}
        ModalProps={{ keepMounted: true }}
        slotProps={{ paper: { sx: drawerSx } }}
      />
    );
  }

  const direction = txDirection(row);
  const isIncoming = direction === 'in';
  const invoice = row.transaction_type === TransactionType.INVOICE ? (row as Invoice) : null;
  const payment = row.transaction_type === TransactionType.PAYMENT ? (row as Payment) : null;
  const invoiceOutpoint = invoice?.bitcoin_output?.outpoint;
  const methodLabel = getLedgerLabel(row.ledger, t);
  const feeAmount = row.fee_msat || 0;
  const amountOnly = row.amount_msat || 0;
  const totalAmount = txAmount(row);
  const isOpenAmount = isOpenAmountRequest(row);
  const invoiceBolt11 = invoice?.ln_invoice?.bolt11;
  const bitcoinDestination =
    payment?.internal?.btc_address || payment?.bitcoin?.address || invoice?.bitcoin_output?.address;
  const destination =
    payment?.lightning?.ln_address ||
    payment?.internal?.ln_address ||
    bitcoinDestination ||
    payment?.bitcoin?.txid ||
    invoiceBolt11;
  let destinationLabel = destination;
  if (destination && destination === bitcoinDestination) {
    destinationLabel = compactBitcoinAddress(destination);
  }
  if (destination && destination === invoiceBolt11) {
    destinationLabel = compactIdentifier(destination);
  }
  const explorerUrl = bitcoinTransactionExplorerUrl(
    payment?.bitcoin?.txid || txidFromOutpoint(invoiceOutpoint)
  );

  return (
    <Drawer
      anchor="right"
      open={!!row}
      onClose={onClose}
      transitionDuration={{ enter: 225, exit: 195 }}
      ModalProps={{ keepMounted: true }}
      slotProps={{ paper: { sx: drawerSx } }}
    >
      <Box>
        <Stack
          direction="row"
          spacing={2}
          sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
        >
          <Stack sx={{ minWidth: 0 }}>
            <Typography variant="h6" noWrap>
              {title || txDisplayTitle(row, t)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {isIncoming ? t('wallet_view.direction_in') : t('wallet_view.direction_out')} ·{' '}
              {methodLabel}
            </Typography>
          </Stack>

          <IconButton onClick={onClose} aria-label={t('close')}>
            <Iconify icon="mingcute:close-line" />
          </IconButton>
        </Stack>

        <Divider />

        <Stack spacing={3} sx={{ p: 3 }}>
          <Box
            sx={[
              (theme) => ({
                p: 2,
                borderRadius: 1,
                bgcolor: 'background.neutral',
                border: `1px solid ${theme.vars.palette.divider}`,
              }),
            ]}
          >
            <Stack spacing={1.5}>
              <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                <Label color={isIncoming ? 'success' : 'warning'}>
                  {isIncoming ? t('wallet_view.direction_in') : t('wallet_view.direction_out')}
                </Label>
                <Label variant="soft" color={statusColor(row.status)}>
                  {row.status}
                </Label>
              </Stack>

              {isOpenAmount ? (
                <Typography variant="h4">{t('wallet_view.open_amount')}</Typography>
              ) : (
                <SatsWithIcon amountMSats={totalAmount} variant="h4" sx={{ fontWeight: 400 }} />
              )}
            </Stack>
          </Box>

          <Grid container spacing={{ xs: 1, sm: 1.5 }}>
            <Grid size={{ xs: 4 }}>
              <QuickDrawerMetric
                label={t('wallet_view.amount')}
                amountMSats={isOpenAmount ? undefined : amountOnly}
                value={isOpenAmount ? t('wallet_view.open_amount') : undefined}
              />
            </Grid>
            <Grid size={{ xs: 4 }}>
              <QuickDrawerMetric
                label={t('wallet_view.fee')}
                amountMSats={feeAmount}
                showMillisatsTooltip
              />
            </Grid>
            <Grid size={{ xs: 4 }}>
              <QuickDrawerMetric
                label={t('wallet_view.total')}
                amountMSats={isOpenAmount ? undefined : totalAmount}
                value={isOpenAmount ? t('wallet_view.open_amount') : undefined}
              />
            </Grid>
          </Grid>

          <Stack spacing={1.5}>
            <QuickDrawerRow label={t('wallet_view.rail')} value={methodLabel} />
            <QuickDrawerRow label={t('wallet_view.created')} value={fDateTime(row.created_at)} />
            {invoice?.ln_invoice?.expires_at && (
              <QuickDrawerRow
                label={t('transaction_list.expires')}
                value={fDateTime(invoice.ln_invoice.expires_at)}
              />
            )}
            <QuickDrawerRow
              label={t('wallet_view.settled')}
              value={row.payment_time ? fDateTime(row.payment_time) : t('wallet_view.not_settled')}
            />
            {destination && (
              <QuickDrawerCopyRow
                label={t('wallet_view.destination')}
                value={destinationLabel || destination}
                copyValue={destination}
                copyLabel={t('transaction_actions.copy_destination')}
              />
            )}
            <QuickDrawerCopyRow
              label={t('wallet_view.transaction_id')}
              value={compactIdentifier(row.id)}
              copyValue={row.id}
              copyLabel={t('activity_view.copy_transaction_id')}
            />
          </Stack>

          {explorerUrl && (
            <Button
              component="a"
              href={explorerUrl}
              target="_blank"
              rel="noopener noreferrer"
              color="inherit"
              variant="outlined"
              startIcon={<Iconify icon="solar:map-arrow-right-bold" />}
            >
              {t('transaction_actions.open_explorer')}
            </Button>
          )}

          {detailHref && (
            <Button
              href={detailHref}
              color="inherit"
              variant="outlined"
              startIcon={<Iconify icon="solar:bill-list-bold-duotone" />}
            >
              {t('wallet_view.open_details')}
            </Button>
          )}

          {canDelete && onDeleteRow && (
            <Button
              color="error"
              variant="outlined"
              onClick={confirm.onTrue}
              startIcon={<Iconify icon="solar:trash-bin-trash-bold" />}
            >
              {t('delete')}
            </Button>
          )}
        </Stack>

        <ConfirmDialog
          open={confirm.value}
          onClose={confirm.onFalse}
          title={t('delete')}
          content={t('confirm_delete')}
          action={
            <Button
              variant="contained"
              color="error"
              loading={isDeleting.value}
              onClick={async () => {
                if (!onDeleteRow) return;
                isDeleting.onTrue();
                await onDeleteRow();
                isDeleting.onFalse();
                confirm.onFalse();
              }}
            >
              {t('delete')}
            </Button>
          }
        />
      </Box>
    </Drawer>
  );
}

function QuickDrawerMetric({
  label,
  amountMSats,
  value,
  showMillisatsTooltip = false,
}: {
  label: string;
  amountMSats?: number;
  value?: string;
  showMillisatsTooltip?: boolean;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: 1.5,
          height: 1,
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
        }),
      ]}
    >
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      {value ? (
        <Typography variant="subtitle2">{value}</Typography>
      ) : (
        <SatsWithIcon
          amountMSats={amountMSats ?? 0}
          variant="subtitle2"
          showMillisatsTooltip={showMillisatsTooltip}
        />
      )}
    </Box>
  );
}

function QuickDrawerRow({
  label,
  value,
  mono,
}: {
  label: string;
  value?: string | null;
  mono?: boolean;
}) {
  if (!value) return null;

  return (
    <Stack direction="row" spacing={1.5} sx={{ justifyContent: 'space-between', minWidth: 0 }}>
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Typography
        variant="body2"
        sx={{
          maxWidth: '68%',
          textAlign: 'right',
          wordBreak: 'break-word',
          fontFamily: mono ? 'monospace' : undefined,
        }}
      >
        {value}
      </Typography>
    </Stack>
  );
}

function QuickDrawerCopyRow({
  label,
  value,
  copyValue,
  copyLabel,
}: {
  label: string;
  value: string;
  copyValue: string;
  copyLabel: string;
}) {
  const { t } = useTranslate();
  const { copy } = useCopyToClipboard();

  const handleCopy = async () => {
    if (await copy(copyValue)) {
      toast.success(t('copied_to_clipboard'));
    }
  };

  return (
    <ButtonBase
      type="button"
      title={copyLabel}
      aria-label={copyLabel}
      onClick={handleCopy}
      sx={[
        (theme) => ({
          py: 0.5,
          gap: 1.5,
          width: 1,
          display: 'flex',
          minWidth: 0,
          borderRadius: 0.75,
          textAlign: 'left',
          alignItems: 'center',
          justifyContent: 'space-between',
          transition: theme.transitions.create('background-color'),
          '&:hover': {
            bgcolor: 'action.hover',
          },
        }),
      ]}
    >
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center', minWidth: 0 }}>
        <Typography
          variant="body2"
          sx={{ typography: 'caption', fontFamily: 'monospace', color: 'text.primary' }}
          noWrap
        >
          {value}
        </Typography>
        <Iconify icon="eva:copy-fill" width={16} sx={{ flexShrink: 0, color: 'text.disabled' }} />
      </Stack>
    </ButtonBase>
  );
}
