import { mutate } from 'swr';
import { useCallback } from 'react';

import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import TableBody from '@mui/material/TableBody';
import Typography from '@mui/material/Typography';
import { Card, Alert, Button, MenuList, MenuItem, Collapse, IconButton } from '@mui/material';

import { useBoolean } from 'src/hooks/use-boolean';

import { fFromNow } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { revokeWalletApiKey, type ApiKeyResponse, type ListWalletApiKeysResponse } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { TableHeadCustom } from 'src/components/table';
import { CreateApiKeyDialog } from 'src/components/api-key';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

const TABLE_HEAD = [
  { id: 'permissions', label: 'Scopes' },
  { id: 'name', label: 'Name' },
  { id: 'description', label: 'Description' },
  { id: 'created_at', label: 'Created' },
  { id: 'expires_at', label: 'Expires in' },
  { id: '' },
];

type Props = {
  apiKeys: ListWalletApiKeysResponse;
};

export function SettingsApiKey({ apiKeys }: Props) {
  const { t } = useTranslate();
  const newApiKey = useBoolean();

  return (
    <Card sx={{ p: { xs: 1, sm: 3 }, mx: 'auto' }}>
      <Stack direction="row" alignItems="center" justifyContent="space-between" sx={{ mb: 3 }}>
        <Typography variant="h5">{t('settings_api_key.title')}</Typography>

        <Button onClick={newApiKey.onTrue} variant="contained" startIcon={<Iconify icon="mingcute:add-line" />}>
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
          <TableHeadCustom headLabel={TABLE_HEAD} />

          <TableBody>
            {apiKeys.map((row) => (
              <CollapsibleTableRow key={row.id} row={row} />
            ))}
          </TableBody>
        </Table>
      </Scrollbar>

      <CreateApiKeyDialog
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
  row: ApiKeyResponse;
};

function CollapsibleTableRow({ row }: CollapsibleTableRowProps) {
  const collapsible = useBoolean();
  const { t } = useTranslate();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  const handleDeleteRow = useCallback(async () => {
    isDeleting.onTrue();

    try {
      await revokeWalletApiKey({ path: { id: row.id } });

      toast.success(t('settings_api_key.revoke_success'));
      mutate(endpointKeys.userWallet.apiKeys.list);
    } catch (error) {
      toast.error(error.reason);
    } finally {
      isDeleting.onFalse();
      confirm.onFalse();
    }
  }, [t, isDeleting, confirm, row]);

  return (
    <>
      <TableRow sx={{ '& > *': { borderBottom: 'unset' } }}>
        <TableCell>
          <IconButton size="small" color={collapsible.value ? 'inherit' : 'default'} onClick={collapsible.onToggle}>
            <Iconify icon={collapsible.value ? 'eva:arrow-ios-upward-fill' : 'eva:arrow-ios-downward-fill'} />
          </IconButton>
        </TableCell>
        <TableCell>
          <b>{row.name}</b>
        </TableCell>
        <TableCell>{row.description}</TableCell>
        <TableCell>{fFromNow(row.created_at)}</TableCell>
        <TableCell>{row.expires_at ? fFromNow(row.expires_at) : 'never'}</TableCell>
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
          <LoadingButton variant="contained" color="error" onClick={handleDeleteRow} loading={isDeleting.value}>
            {t('settings_api_key.revoke')}
          </LoadingButton>
        }
      />
    </>
  );
}
