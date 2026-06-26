import { mutate } from 'swr';
import { useCallback } from 'react';
import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import TableBody from '@mui/material/TableBody';
import Typography from '@mui/material/Typography';
import { Card, Alert, Button, MenuList, MenuItem, Collapse, IconButton } from '@mui/material';

import { fFromNow } from 'src/utils/format-time';
import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import {
  type ApiKey,
  revokeWalletApiKey,
  type ListWalletApiKeysResponse,
} from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { TableHeadCustom } from 'src/components/table';
import { CreateApiKeyDrawer } from 'src/components/api-key';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

type Props = {
  apiKeys: ListWalletApiKeysResponse;
};

export function SettingsApiKey({ apiKeys }: Props) {
  const { t } = useTranslate();
  const newApiKey = useBoolean();
  const tableHead = [
    { id: 'permissions', label: t('api_key_list.scopes') },
    { id: 'name', label: t('api_key_list.name') },
    { id: 'description', label: t('api_key_list.description') },
    { id: 'created_at', label: t('api_key_list.created') },
    { id: 'expires_at', label: t('api_key_list.expires') },
    { id: '' },
  ];

  return (
    <Card sx={{ p: { xs: 1, sm: 3 }, mx: 'auto' }}>
      <Stack direction="row" sx={{ mb: 3, alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="h5">{t('settings_api_key.title')}</Typography>

        <Button
          onClick={newApiKey.onTrue}
          variant="contained"
          startIcon={<Iconify icon="mingcute:add-line" />}
        >
          {t('new')}
        </Button>
      </Stack>

      <Typography variant="body1" sx={{ mb: 3, color: 'text.secondary' }}>
        {t('settings_api_key.description')}
      </Typography>

      <Alert severity="info" sx={{ mb: 3 }} variant="outlined">
        <Typography variant="body2">
          <strong>{t('important')}:</strong> {t('settings_api_key.alert')}
        </Typography>
      </Alert>

      <Scrollbar>
        <Table sx={{ minWidth: 800 }}>
          <TableHeadCustom headCells={tableHead} />

          <TableBody>
            {apiKeys.map((row) => (
              <CollapsibleTableRow key={row.id} row={row} />
            ))}
          </TableBody>
        </Table>
      </Scrollbar>

      <CreateApiKeyDrawer
        title={t('settings_api_key.new_dialog_title')}
        open={newApiKey.value}
        onClose={newApiKey.onFalse}
        onSuccess={() => {
          mutate(endpointKeys.userWallet.apiKeys.list);
        }}
      />
    </Card>
  );
}

// ----------------------------------------------------------------------

type CollapsibleTableRowProps = {
  row: ApiKey;
};

function CollapsibleTableRow({ row }: CollapsibleTableRowProps) {
  const collapsible = useBoolean();
  const { t } = useTranslate();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();
  const scopeCount = row.permissions.length + 1;

  const handleDeleteRow = useCallback(async () => {
    isDeleting.onTrue();

    try {
      await revokeWalletApiKey({ path: { id: row.id } });

      toast.success(t('settings_api_key.revoke_success'));
      mutate(endpointKeys.userWallet.apiKeys.list);
    } catch (error) {
      handleActionError(error);
    } finally {
      isDeleting.onFalse();
      confirm.onFalse();
    }
  }, [t, isDeleting, confirm, row]);

  return (
    <>
      <TableRow sx={{ '& > *': { borderBottom: 'unset' } }}>
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
          <b>{row.name}</b>
        </TableCell>
        <TableCell>{row.description}</TableCell>
        <TableCell>{fFromNow(row.created_at)}</TableCell>
        <TableCell>{row.expires_at ? fFromNow(row.expires_at) : t('api_key_list.never')}</TableCell>
        <TableCell>
          <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </TableCell>
      </TableRow>

      <TableRow>
        <TableCell sx={{ py: 0 }} colSpan={6}>
          <Collapse in={collapsible.value} timeout="auto" unmountOnExit>
            <Stack direction="row" spacing={1} sx={{ my: 2 }}>
              <Label variant="soft" color="secondary">
                {t('api_key_list.user_wallet_permission')}
              </Label>
              {row.permissions.map((scope) => (
                <Label key={scope} variant="soft" color="default">
                  {scope}
                </Label>
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
            {t('settings_api_key.revoke')}
          </MenuItem>
        </MenuList>
      </CustomPopover>

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('settings_api_key.revoke')}
        content={t('settings_api_key.confirm_revoke')}
        action={
          <Button
            variant="contained"
            color="error"
            onClick={handleDeleteRow}
            loading={isDeleting.value}
          >
            {t('settings_api_key.revoke')}
          </Button>
        }
      />
    </>
  );
}
