import type { CardProps } from '@mui/material/Card';
import type { IBreezLSP } from 'src/types/breez-node';

import Link from 'next/link';
import { mutate } from 'swr';
import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import { LoadingButton } from '@mui/lab';
import Avatar from '@mui/material/Avatar';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import TableBody from '@mui/material/TableBody';
import IconButton from '@mui/material/IconButton';
import CardHeader from '@mui/material/CardHeader';
import ListItemText from '@mui/material/ListItemText';
import TableContainer from '@mui/material/TableContainer';
import { Button, Dialog, Divider, MenuList, Typography, DialogTitle, DialogActions, DialogContent, Link as MuiLink } from '@mui/material';

import { useBoolean } from 'src/hooks/use-boolean';

import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { connectLsp, closeLspChannels } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { TableHeadCustom } from 'src/components/table';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

interface Props extends CardProps {
  title?: string;
  subheader?: string;
  tableData: IBreezLSP[];
  tableLabels: any;
  currentLSP: string;
}

export function LSPList({ title, subheader, currentLSP, tableLabels, tableData, ...other }: Props) {
  return (
    <Card {...other}>
      <CardHeader title={title} subheader={subheader} sx={{ mb: 3, textTransform: 'capitalize' }} />

      <TableContainer sx={{ overflow: 'unset' }}>
        <Scrollbar>
          <Table sx={{ minWidth: 720 }}>
            <TableHeadCustom headLabel={tableLabels} />

            <TableBody>
              {tableData.map((row) => (
                <LSPRow key={row.id} row={row} currentLSP={currentLSP} />
              ))}
            </TableBody>
          </Table>
        </Scrollbar>
      </TableContainer>
    </Card>
  );
}

// ----------------------------------------------------------------------

type LSPRowProps = {
  row: IBreezLSP;
  currentLSP: string;
};

function LSPRow({ row, currentLSP }: LSPRowProps) {
  const { id, name, pubkey, host, base_fee_msat, fee_rate, time_lock_delta, min_htlc_msat } = row;

  const { t } = useTranslate();
  const popover = usePopover();
  const confirmConnect = useBoolean();
  const confirmCloseChannels = useBoolean();
  const isConnecting = useBoolean();
  const isClosingChannels = useBoolean();
  const closingChannelDialog = useBoolean();
  const [txids, setTxids] = useState<string[]>([]);

  const mempoolHref = `https://mempool.space/lightning/node/${pubkey}`;
  const active = currentLSP === pubkey;

  const handleConnect = useCallback(async () => {
    isConnecting.onTrue();

    const { error } = await connectLsp({ body: { lsp_id: id } });
    if (error) {
      toast.error(error.reason);
      isConnecting.onFalse();
      return;
    }

    mutate(endpointKeys.lightning.node.info);
    confirmConnect.onFalse();
    toast.success(t('lsp_list.connect_success'));
    isConnecting.onFalse();
  }, [id, confirmConnect, isConnecting, t]);

  const handleCloseChannels = useCallback(async () => {
    isClosingChannels.onTrue();

    const { error, data } = await closeLspChannels();

    if (error) {
      toast.error(error.reason);
      isClosingChannels.onFalse();
      return;
    }

    mutate(endpointKeys.lightning.node.info);
    confirmCloseChannels.onFalse();

    if (data.length) {
      setTxids(data);
      closingChannelDialog.onTrue();
    } else {
      toast.info(t('lsp_list.no_channels_to_close'));
    }

    isClosingChannels.onFalse();
  }, [confirmCloseChannels, closingChannelDialog, isClosingChannels, t]);

  return (
    <>
      <TableRow>
        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <Box sx={{ position: 'relative', mr: 2 }}>
            <Avatar
              alt={name}
              sx={{
                width: 48,
                height: 48,
                color: 'text.secondary',
                bgcolor: 'background.neutral',
              }}
            >
              {name.charAt(0).toUpperCase()}
            </Avatar>
          </Box>
          <ListItemText
            disableTypography
            primary={
              <Typography variant="body2" noWrap>
                {truncateText(name, 20)}
              </Typography>
            }
            secondary={
              <MuiLink noWrap variant="body2" href={mempoolHref} target="_blank" sx={{ color: 'text.disabled', cursor: 'pointer' }}>
                {truncateText(pubkey, 15)}
              </MuiLink>
            }
          />
        </TableCell>

        <TableCell>
          <Typography>{truncateText(id, 15)}</Typography>
        </TableCell>

        <TableCell>
          <Typography>{host}</Typography>
        </TableCell>

        <TableCell>
          <SatsWithIcon amountMSats={base_fee_msat} />
        </TableCell>

        <TableCell>
          <Typography>{fee_rate.toExponential()}</Typography>
        </TableCell>

        <TableCell>
          <Typography>{time_lock_delta}</Typography>
        </TableCell>

        <TableCell>
          <Typography>{min_htlc_msat}</Typography>
        </TableCell>

        <TableCell>
          <Label variant="soft" color={(active && 'success') || 'default'}>
            {active ? t('lsp_list.active') : t('lsp_list.inactive')}
          </Label>
        </TableCell>

        <TableCell align="right" sx={{ pr: 1 }}>
          <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </TableCell>
      </TableRow>

      <CustomPopover open={popover.open} anchorEl={popover.anchorEl} onClose={popover.onClose}>
        <MenuList>
          <Link href={mempoolHref} passHref target="blank" style={{ textDecoration: 'none', color: 'inherit' }}>
            <MenuItem>
              <Iconify icon="eva:eye-fill" />
              {t('details')}
            </MenuItem>
          </Link>

          <CopyMenuItem value={pubkey} title={t('lsp_list.copy_pubkey')} />
          <CopyMenuItem value={host} title={t('lsp_list.copy_host')} />

          {!active && (
            <div>
              <Divider sx={{ borderStyle: 'dashed' }} />

              <MenuItem
                onClick={() => {
                  confirmConnect.onTrue();
                  popover.onClose();
                }}
                sx={{ color: 'primary.light' }}
              >
                <Iconify icon="eva:link-2-fill" />
                {t('lsp_list.connect')}
              </MenuItem>
            </div>
          )}

          {active && (
            <div>
              <Divider sx={{ borderStyle: 'dashed' }} />

              <MenuItem
                onClick={() => {
                  confirmCloseChannels.onTrue();
                  popover.onClose();
                }}
                sx={{ color: 'error.main' }}
              >
                <Iconify icon="eva:close-circle-fill" />
                {t('lsp_list.close_channels')}
              </MenuItem>
            </div>
          )}
        </MenuList>
      </CustomPopover>

      <ConfirmDialog
        open={confirmConnect.value}
        onClose={confirmConnect.onFalse}
        title={t('lsp_list.confirm_connect.title')}
        content={t('lsp_list.confirm_connect.content', { name })}
        action={
          <LoadingButton variant="contained" color="warning" onClick={handleConnect} loading={isConnecting.value}>
            {t('lsp_list.confirm_connect.connect_button')}
          </LoadingButton>
        }
      />

      <ConfirmDialog
        open={confirmCloseChannels.value}
        onClose={confirmCloseChannels.onFalse}
        title={t('lsp_list.confirm_close_channels.title')}
        content={t('lsp_list.confirm_close_channels.content', { name })}
        action={
          <LoadingButton variant="contained" color="error" onClick={handleCloseChannels} loading={isClosingChannels.value}>
            {t('lsp_list.confirm_close_channels.close_button')}
          </LoadingButton>
        }
      />

      <Dialog open={closingChannelDialog.value} onClose={closingChannelDialog.onFalse}>
        <DialogTitle>{t('lsp_list.closing_dialog.title')}</DialogTitle>

        <DialogContent sx={{ color: 'text.secondary' }}>
          {t('lsp_list.closing_dialog.content')}
          <br />
          {txids.map((txid) => (
            <Typography key={txid} variant="body2">
              <MuiLink key={txid} fontWeight="bold" href={`https://mempool.space/tx/${txid}`} target="_blank">
                - {txid}
              </MuiLink>
            </Typography>
          ))}
        </DialogContent>

        <DialogActions>
          <Button variant="contained" onClick={closingChannelDialog.onFalse} autoFocus>
            {t('close')}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
