import type { ApiKey } from 'src/lib/swissknife';

import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import { Stack, Avatar, Tooltip, MenuList, Collapse } from '@mui/material';

import { fDate, fTime } from 'src/utils/format-time';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

type Props = {
  row: ApiKey;
  selected: boolean;
  onSelectRow: VoidFunction;
  onDeleteRow: () => Promise<void>;
};

export function ApiKeyTableRow({ row, selected, onSelectRow, onDeleteRow }: Props) {
  const { id, account_id, name, description, permissions, created_at, expires_at } = row;

  const { t } = useTranslate();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();
  const collapsible = useBoolean();
  const scopeCount = permissions.length + 1;

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <Avatar alt={account_id} sx={{ mr: 2 }}>
            {account_id.charAt(0).toUpperCase()}
          </Avatar>

          <ListItemText
            disableTypography
            primary={
              <Typography variant="body2" noWrap>
                {truncateText(account_id, 20)}
              </Typography>
            }
            secondary={
              <Tooltip title={id} arrow>
                <Typography noWrap variant="body2" sx={{ color: 'text.disabled' }}>
                  {truncateText(id, 15)}
                </Typography>
              </Tooltip>
            }
          />
        </TableCell>

        <TableCell>
          <Typography variant="body2" noWrap>
            {name}
          </Typography>
        </TableCell>

        <TableCell>
          <Tooltip title={description} arrow>
            <Typography variant="body2" noWrap>
              {truncateText(description, 30)}
            </Typography>
          </Tooltip>
        </TableCell>

        <TableCell>
          <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
            <IconButton
              size="small"
              color={collapsible.value ? 'inherit' : 'default'}
              onClick={collapsible.onToggle}
            >
              <Iconify
                icon={
                  collapsible.value ? 'eva:arrow-ios-upward-fill' : 'eva:arrow-ios-downward-fill'
                }
              />
            </IconButton>
            <Label variant="soft" color="info">
              {t('api_key_list.scope_count', { count: scopeCount })}
            </Label>
          </Stack>
        </TableCell>

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

        <TableCell>
          {expires_at ? (
            <ListItemText
              primary={fDate(expires_at)}
              secondary={fTime(expires_at)}
              slotProps={{
                primary: { noWrap: true, sx: { typography: 'body2' } },
                secondary: {
                  component: 'span',
                  sx: { mt: 0.5, typography: 'caption' },
                },
              }}
            />
          ) : (
            '-'
          )}
        </TableCell>

        <TableCell align="right" sx={{ px: 1 }}>
          <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </TableCell>
      </TableRow>

      <TableRow>
        <TableCell sx={{ py: 0 }} colSpan={8}>
          <Collapse in={collapsible.value} timeout="auto" unmountOnExit>
            <Stack direction="row" spacing={0.5} sx={{ my: 2, flexWrap: 'wrap' }}>
              <Label color="secondary">{t('api_key_list.account_wallet_access')}</Label>

              {permissions.map((permission, i) => (
                <Label key={i}>{permission}</Label>
              ))}
            </Stack>
          </Collapse>
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
