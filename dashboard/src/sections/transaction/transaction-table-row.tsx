import type { ITransaction } from 'src/types/transaction';
import type { InvoiceResponse, PaymentResponse } from 'src/lib/swissknife';

import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Link from '@mui/material/Link';
import { LoadingButton } from '@mui/lab';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import { Avatar, Divider, MenuList } from '@mui/material';

import { useRouter } from 'src/routes/hooks';

import { fDate, fTime } from 'src/utils/format-time';
import { truncateText } from 'src/utils/format-string';

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

  const avatarLetter = (text?: string | null) => (text || id).charAt(0).toUpperCase();

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          {transactionType === TransactionType.PAYMENT && (
            <Avatar alt={id} sx={{ mr: 2 }}>
              {avatarLetter((row as PaymentResponse).ln_address || id)}
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
              <Link
                noWrap
                variant="body2"
                href={href}
                sx={{ color: 'text.disabled', cursor: 'pointer' }}
              >
                {truncateText(id, 15)}
              </Link>
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
            primaryTypographyProps={{ typography: 'body2', noWrap: true }}
            secondaryTypographyProps={{
              mt: 0.5,
              component: 'span',
              typography: 'caption',
            }}
          />
        </TableCell>

        {transactionType === TransactionType.INVOICE && (
          <TableCell>
            {row.payment_time ? (
              '-'
            ) : (
              <ListItemText
                primary={fDate((row as InvoiceResponse).ln_invoice?.expires_at)}
                secondary={fTime((row as InvoiceResponse).ln_invoice?.expires_at)}
                primaryTypographyProps={{ typography: 'body2', noWrap: true }}
                secondaryTypographyProps={{
                  mt: 0.5,
                  component: 'span',
                  typography: 'caption',
                }}
              />
            )}
          </TableCell>
        )}

        <TableCell>
          <ListItemText
            primary={fDate(payment_time)}
            secondary={fTime(payment_time)}
            primaryTypographyProps={{ typography: 'body2', noWrap: true }}
            secondaryTypographyProps={{
              mt: 0.5,
              component: 'span',
              typography: 'caption',
            }}
          />
        </TableCell>

        <TableCell>
          <SatsWithIcon amountMSats={amount_msat || 0} />
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
            {ledger}
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
          <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
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
          {transactionType === TransactionType.INVOICE &&
            status === 'Pending' &&
            (row as InvoiceResponse).ln_invoice && (
              <CopyMenuItem value={(row as InvoiceResponse).ln_invoice!.bolt11} />
            )}

          {isAdmin && (
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
          <LoadingButton
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
          </LoadingButton>
        }
      />
    </>
  );
}
