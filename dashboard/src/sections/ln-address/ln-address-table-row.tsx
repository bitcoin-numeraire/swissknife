import type { LnAddress } from 'src/lib/swissknife';

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

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { npub } from 'src/utils/nostr';
import { displayLnAddress } from 'src/utils/lnurl';
import { fDate, fTime } from 'src/utils/format-time';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

type Props = {
  row: LnAddress;
  selected: boolean;
  onSelectRow: VoidFunction;
  onDeleteRow: () => Promise<void>;
};

export function LnAddressTableRow({ row, selected, onSelectRow, onDeleteRow }: Props) {
  const { t } = useTranslate();
  const { id, wallet_id, username, created_at, updated_at, active, nostr_pubkey, allows_nostr } =
    row;

  const router = useRouter();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <Avatar alt={username} sx={{ mr: 2 }}>
            {username.charAt(0).toUpperCase()}
          </Avatar>

          <ListItemText
            disableTypography
            primary={
              <Typography variant="body2" noWrap>
                {username}
              </Typography>
            }
            secondary={
              <Link
                noWrap
                variant="body2"
                href={paths.admin.lnAddress(id)}
                sx={{ color: 'text.disabled', cursor: 'pointer' }}
              >
                {truncateText(id, 15)}
              </Link>
            }
          />
        </TableCell>

        <TableCell>
          <Typography noWrap variant="body2" sx={{ color: 'text.secondary' }}>
            {truncateText(wallet_id, 15)}
          </Typography>
        </TableCell>

        <TableCell>
          <Typography noWrap variant="body2" sx={{ color: 'text.secondary' }}>
            {truncateText(npub(nostr_pubkey), 15)}
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

        <TableCell>
          <ListItemText
            primary={fDate(updated_at)}
            secondary={fTime(updated_at)}
            primaryTypographyProps={{ typography: 'body2', noWrap: true }}
            secondaryTypographyProps={{
              mt: 0.5,
              component: 'span',
              typography: 'caption',
            }}
          />
        </TableCell>

        <TableCell>
          <Label variant="soft" color={active ? 'success' : 'error'}>
            {active ? t('ln_address_table_row.active') : t('ln_address_table_row.inactive')}
          </Label>
        </TableCell>

        <TableCell>
          <Label variant="soft" color={allows_nostr ? 'success' : 'error'}>
            {allows_nostr ? t('ln_address_table_row.active') : t('ln_address_table_row.inactive')}
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
              router.push(paths.admin.lnAddress(id));
              popover.onClose();
            }}
          >
            <Iconify icon="solar:eye-bold" />
            {t('view')}
          </MenuItem>

          <CopyMenuItem value={displayLnAddress(username)} />
          {nostr_pubkey && <CopyMenuItem value={npub(nostr_pubkey)} title="Copy Nostr npub" />}

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
