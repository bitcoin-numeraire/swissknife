import type { BtcAddress } from 'src/lib/swissknife';

import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Link from '@mui/material/Link';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import { Divider, MenuList } from '@mui/material';

import { paths } from 'src/routes/paths';
import { RouterLink } from 'src/routes/components';
import { useRouter } from 'src/routes/hooks';

import { fDate, fTime } from 'src/utils/format-time';
import { truncateText } from 'src/utils/format-string';
import { compactBitcoinAddress } from 'src/utils/bitcoin-request';
import { bitcoinAddressExplorerUrl } from 'src/utils/bitcoin-explorer';

import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton, CopyMenuItem } from 'src/components/copy';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

type Props = {
  row: BtcAddress;
  selected: boolean;
  onSelectRow: VoidFunction;
  onDeleteRow: () => Promise<void>;
};

export function addressTypeLabel(address: BtcAddress) {
  if (address.address_type === 'p2tr') return 'Taproot';
  if (address.address_type === 'p2wpkh') return 'Native SegWit';
  if (address.address_type === 'p2pkh') return 'Legacy';
  if (address.address_type === 'p2sh') return 'P2SH';
  return address.address_type.toUpperCase();
}

export function BtcAddressTableRow({ row, selected, onSelectRow, onDeleteRow }: Props) {
  const { id, address, wallet_id, created_at, updated_at, used } = row;
  const { t } = useTranslate();
  const router = useRouter();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();
  const explorerUrl = bitcoinAddressExplorerUrl(address);

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <ListItemText
            disableTypography
            primary={
              <Typography variant="body2" sx={{ fontFamily: 'monospace' }} noWrap>
                {compactBitcoinAddress(address)}
              </Typography>
            }
            secondary={
              <Typography noWrap variant="body2" sx={{ color: 'text.disabled' }}>
                {truncateText(id, 15)}
              </Typography>
            }
          />
          <CopyButton value={address} title={t('copy')} />
        </TableCell>

        <TableCell>
          <Link
            noWrap
            component={RouterLink}
            href={paths.account(wallet_id)}
            variant="body2"
            sx={{ color: 'text.secondary' }}
          >
            {truncateText(wallet_id, 15)}
          </Link>
        </TableCell>

        <TableCell>
          <Typography noWrap variant="body2" sx={{ color: 'text.secondary' }}>
            {addressTypeLabel(row)}
          </Typography>
        </TableCell>

        <TableCell>
          <ListItemText
            primary={fDate(created_at)}
            secondary={fTime(created_at)}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: { component: 'span', sx: { mt: 0.5, typography: 'caption' } },
            }}
          />
        </TableCell>

        <TableCell>
          <ListItemText
            primary={updated_at ? fDate(updated_at) : 'N/A'}
            secondary={updated_at ? fTime(updated_at) : undefined}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: { component: 'span', sx: { mt: 0.5, typography: 'caption' } },
            }}
          />
        </TableCell>

        <TableCell>
          <Label variant="soft" color={used ? 'warning' : 'success'}>
            {used ? t('btc_address_list.used') : t('btc_address_list.unused')}
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
              router.push(paths.account(wallet_id));
              popover.onClose();
            }}
          >
            <Iconify icon="solar:safe-square-bold-duotone" />
            {t('wallets')}
          </MenuItem>

          <CopyMenuItem value={address} title={t('copy')} />
          <CopyMenuItem value={wallet_id} title={t('btc_address_table_row.copy_wallet_id')} />

          {explorerUrl && (
            <MenuItem component="a" href={explorerUrl} target="_blank" rel="noopener noreferrer">
              <Iconify icon="solar:map-arrow-right-bold" />
              {t('transaction_actions.open_explorer')}
            </MenuItem>
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
