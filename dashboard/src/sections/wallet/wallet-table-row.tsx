import type { IFiatPrices } from 'src/types/bitcoin';
import type { WalletOverview } from 'src/lib/swissknife';

import { useState } from 'react';
import { useBoolean, usePopover } from 'minimal-shared/hooks';

import { LoadingButton } from '@mui/lab';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import { Link, Avatar, Divider, MenuList } from '@mui/material';

import { paths } from 'src/routes/paths';

import { displayLnAddress } from 'src/utils/lnurl';
import { fDate, fTime } from 'src/utils/format-time';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';
import {
  NewInvoiceDialog,
  NewPaymentDialog,
  ConfirmPaymentDialog,
} from 'src/components/transactions';

// ----------------------------------------------------------------------

type Props = {
  row: WalletOverview;
  selected: boolean;
  onSelectRow: VoidFunction;
  onDeleteRow: () => Promise<void>;
  fiatPrices: IFiatPrices;
};

export function WalletTableRow({ row, selected, onSelectRow, onDeleteRow, fiatPrices }: Props) {
  const { id, user_id, ln_address, n_contacts, n_invoices, n_payments, balance, created_at } = row;

  const { t } = useTranslate();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();
  const newPayment = useBoolean();
  const newInvoice = useBoolean();
  const sendTo = useBoolean();
  const [input, setInput] = useState('');

  const handleClickSendTo = (val: string) => {
    setInput(displayLnAddress(val));
    sendTo.onTrue();
  };

  const handleCloseSendTo = () => {
    setInput('');
    sendTo.onFalse();
  };

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <Avatar alt={user_id} sx={{ mr: 2 }}>
            {user_id.charAt(0).toUpperCase()}
          </Avatar>

          <ListItemText
            disableTypography
            primary={
              <Typography variant="body2" noWrap>
                {truncateText(user_id, 20)}
              </Typography>
            }
            secondary={
              <Typography noWrap variant="body2" sx={{ color: 'text.disabled' }}>
                {truncateText(id, 15)}
              </Typography>
            }
          />
        </TableCell>

        <TableCell>
          {ln_address ? (
            <ListItemText
              disableTypography
              primary={
                <Typography variant="body2" noWrap>
                  {ln_address.username}
                </Typography>
              }
              secondary={
                <Link
                  noWrap
                  variant="body2"
                  href={paths.admin.lnAddress(ln_address.id || '')}
                  sx={{ color: 'text.disabled', cursor: 'pointer' }}
                >
                  {truncateText(ln_address.id, 15)}
                </Link>
              }
            />
          ) : (
            '-'
          )}
        </TableCell>

        <TableCell>
          <SatsWithIcon amountMSats={balance.available_msat || 0} />
        </TableCell>

        <TableCell>
          <Typography variant="body2" noWrap>
            {n_invoices}
          </Typography>
        </TableCell>

        <TableCell>
          <Typography variant="body2" noWrap>
            {n_payments}
          </Typography>
        </TableCell>

        <TableCell>
          <Typography variant="body2" noWrap>
            {n_contacts}
          </Typography>
        </TableCell>

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
              newPayment.onTrue();
              popover.onClose();
            }}
          >
            <Iconify icon="eva:diagonal-arrow-right-up-fill" />
            {t('wallet_table_row.new_payment')}
          </MenuItem>

          <MenuItem
            onClick={() => {
              newInvoice.onTrue();
              popover.onClose();
            }}
          >
            <Iconify icon="eva:diagonal-arrow-left-down-fill" />
            {t('wallet_table_row.new_invoice')}
          </MenuItem>

          {ln_address && (
            <>
              <CopyMenuItem
                title={t('wallet_table_row.copy_ln_address')}
                value={displayLnAddress(ln_address.username)}
              />
              <MenuItem onClick={() => handleClickSendTo(ln_address.username)}>
                <Iconify icon="eva:flash-fill" />
                {t('wallet_table_row.send_to_ln_address')}
              </MenuItem>
            </>
          )}

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
        </MenuList>
      </CustomPopover>

      <NewPaymentDialog
        isAdmin
        walletId={id}
        balance={balance.available_msat}
        fiatPrices={fiatPrices}
        open={newPayment.value}
        onClose={newPayment.onFalse}
      />

      <NewInvoiceDialog
        isAdmin
        walletId={id}
        fiatPrices={fiatPrices!}
        open={newInvoice.value}
        onClose={newInvoice.onFalse}
      />

      <ConfirmPaymentDialog
        input={input}
        open={sendTo.value}
        onClose={handleCloseSendTo}
        fiatPrices={fiatPrices}
      />

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
