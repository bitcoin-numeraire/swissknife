import type { ITransaction } from 'src/types/transaction';
import type { Invoice, Payment } from 'src/lib/swissknife';

import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import { Avatar, Divider, MenuList } from '@mui/material';

import { useRouter } from 'src/routes/hooks';

import { truncateText } from 'src/utils/format-string';
import { getLedgerLabel } from 'src/utils/transactions';
import { composeBip21 } from 'src/utils/bitcoin-request';
import { fDate, fTime, fDateTime } from 'src/utils/format-time';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

type Props = {
  transactionType: TransactionType;
  row: ITransaction;
  selected: boolean;
  onSelectRow: VoidFunction;
  href: string;
  onDeleteRow: () => Promise<void>;
  isAdmin?: boolean;
};

function txExplorerUrl(txid?: string | null) {
  if (!txid) return undefined;

  const explorerBaseUrl = CONFIG.mempoolSpace.replace(/\/api\/v1\/?$/, '');
  return `${explorerBaseUrl}/tx/${txid}`;
}

export function TransactionTableRow({
  row,
  isAdmin,
  transactionType,
  selected,
  onSelectRow,
  href,
  onDeleteRow,
}: Props) {
  const { id, amount_msat, wallet_id, description, created_at, payment_time, status, ledger } = row;

  const { t } = useTranslate();
  const router = useRouter();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();
  const isPayment = transactionType === TransactionType.PAYMENT;
  const amountColor = isPayment ? 'warning.main' : 'success.main';
  const methodLabel = getLedgerLabel(ledger, t);

  const avatarLetter = (text?: string | null) =>
    (text || description || ledger).charAt(0).toUpperCase();
  const invoice = row as Invoice;
  const payment = row as Payment;
  const invoiceBolt11 =
    transactionType === TransactionType.INVOICE ? invoice.ln_invoice?.bolt11 : undefined;
  const invoiceAddress =
    transactionType === TransactionType.INVOICE ? invoice.bitcoin_output?.address : undefined;
  const invoiceUnified =
    invoiceBolt11 && invoiceAddress
      ? composeBip21(
          invoiceAddress,
          invoiceBolt11,
          invoice.amount_msat ? invoice.amount_msat / 1000 : 0
        )
      : undefined;
  const paymentAddress =
    transactionType === TransactionType.PAYMENT
      ? payment.bitcoin?.address || payment.internal?.btc_address
      : undefined;
  const explorerUrl =
    transactionType === TransactionType.PAYMENT ? txExplorerUrl(payment.bitcoin?.txid) : undefined;
  const canDelete = isAdmin || status === 'Expired' || status === 'Failed';

  return (
    <>
      <TableRow
        hover
        selected={selected}
        onClick={() => router.push(href)}
        sx={{ cursor: 'pointer' }}
      >
        <TableCell padding="checkbox">
          <Checkbox
            checked={selected}
            onClick={(event) => {
              event.stopPropagation();
              onSelectRow();
            }}
          />
        </TableCell>

        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          {isPayment && (
            <Avatar alt={id} sx={{ mr: 2 }}>
              {avatarLetter((row as Payment).lightning?.ln_address || id)}
            </Avatar>
          )}

          <ListItemText
            disableTypography
            primary={
              <Typography variant="body2" sx={{ whiteSpace: 'normal', wordWrap: 'break-word' }}>
                {description || t('recent_transactions.empty_description')}
              </Typography>
            }
            secondary={
              <Typography noWrap variant="body2" sx={{ color: 'text.disabled' }}>
                {fDateTime(created_at)} · {methodLabel}
              </Typography>
            }
          />
        </TableCell>

        {isAdmin && (
          <TableCell>
            <Typography noWrap variant="body2" sx={{ color: 'text.secondary' }}>
              {truncateText(wallet_id, 15)}
            </Typography>
          </TableCell>
        )}

        <TableCell>
          <ListItemText
            primary={fDate(created_at)}
            secondary={fTime(created_at)}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: {
                component: 'span',
                sx: { mt: 0.5, typography: 'caption' },
              },
            }}
          />
        </TableCell>

        {transactionType === TransactionType.INVOICE && (
          <TableCell>
            {row.payment_time ? (
              '-'
            ) : (
              <ListItemText
                primary={fDate((row as Invoice).ln_invoice?.expires_at)}
                secondary={fTime((row as Invoice).ln_invoice?.expires_at)}
                slotProps={{
                  primary: { noWrap: true, sx: { typography: 'body2' } },
                  secondary: {
                    component: 'span',
                    sx: { mt: 0.5, typography: 'caption' },
                  },
                }}
              />
            )}
          </TableCell>
        )}

        <TableCell>
          <ListItemText
            primary={fDate(payment_time)}
            secondary={fTime(payment_time)}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: {
                component: 'span',
                sx: { mt: 0.5, typography: 'caption' },
              },
            }}
          />
        </TableCell>

        <TableCell>
          <Stack direction="row" spacing={0.25} sx={{ alignItems: 'center' }}>
            <Typography component="span" variant="body2" sx={{ color: amountColor }}>
              {isPayment ? '-' : '+'}
            </Typography>
            <SatsWithIcon
              component="span"
              amountMSats={amount_msat || 0}
              sx={{ color: amountColor }}
            />
          </Stack>
        </TableCell>

        <TableCell>
          <Label
            variant="soft"
            color={
              (ledger === 'Lightning' && 'secondary') ||
              (ledger === 'Internal' && 'primary') ||
              'default'
            }
          >
            {methodLabel}
          </Label>
        </TableCell>

        <TableCell>
          <Label
            variant="soft"
            color={
              (status === 'Settled' && 'success') ||
              (status === 'Pending' && 'warning') ||
              (status === 'Expired' && 'error') ||
              'default'
            }
          >
            {status}
          </Label>
        </TableCell>

        <TableCell align="right" sx={{ px: 1 }}>
          <IconButton
            color={popover.open ? 'inherit' : 'default'}
            onClick={(event) => {
              event.stopPropagation();
              popover.onOpen(event);
            }}
          >
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </TableCell>
      </TableRow>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        <MenuList>
          <MenuItem
            onClick={() => {
              router.push(href);
              popover.onClose();
            }}
          >
            <Iconify icon="solar:eye-bold" />
            {t('view')}
          </MenuItem>
          {invoiceUnified && (
            <CopyMenuItem value={invoiceUnified} title={t('transaction_actions.copy_unified')} />
          )}
          {invoiceBolt11 && (
            <CopyMenuItem value={invoiceBolt11} title={t('transaction_actions.copy_bolt11')} />
          )}
          {invoiceAddress && (
            <CopyMenuItem
              value={invoiceAddress}
              title={t('transaction_actions.copy_onchain_address')}
            />
          )}
          {paymentAddress && (
            <CopyMenuItem
              value={paymentAddress}
              title={t('transaction_actions.copy_destination')}
            />
          )}
          {explorerUrl && (
            <MenuItem component="a" href={explorerUrl} target="_blank" rel="noopener noreferrer">
              <Iconify icon="solar:map-arrow-right-bold" />
              {t('transaction_actions.open_explorer')}
            </MenuItem>
          )}

          {canDelete && (
            <>
              <Divider sx={{ borderStyle: 'dashed' }} />

              <MenuItem
                onClick={() => {
                  confirm.onTrue();
                  popover.onClose();
                }}
                sx={{ color: 'error.main' }}
              >
                <Iconify icon="solar:trash-bin-trash-bold" />
                {t('delete')}
              </MenuItem>
            </>
          )}
        </MenuList>
      </CustomPopover>

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('delete')}
        content={t('confirm_delete')}
        action={
          <Button
            variant="contained"
            color="error"
            onClick={async () => {
              isDeleting.onTrue();
              await onDeleteRow();
              isDeleting.onFalse();
            }}
            loading={isDeleting.value}
          >
            {t('delete')}
          </Button>
        }
      />
    </>
  );
}
