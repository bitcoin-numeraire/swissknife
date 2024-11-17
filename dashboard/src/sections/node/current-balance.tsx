import type { CardProps } from '@mui/material/Card';
import type { IBreezNodeInfo } from 'src/types/breez-node';

import { mutate } from 'swr';
import { useState, useCallback } from 'react';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link, Button, Dialog, MenuList, MenuItem, TextField, IconButton, DialogTitle, DialogActions, DialogContent } from '@mui/material';

import { useBoolean } from 'src/hooks/use-boolean';

import { fSats } from 'src/utils/format-number';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { sync, backup, redeem } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

interface Props extends CardProps {
  title: string;
  nodeInfo: IBreezNodeInfo;
}

export function CurrentBalance({ title, nodeInfo, sx, ...other }: Props) {
  const {
    channels_balance_msat,
    max_payable_msat,
    max_receivable_msat,
    onchain_balance_msat,
    inbound_liquidity_msats,
    pending_onchain_balance_msat,
    id,
  } = nodeInfo;

  const { t } = useTranslate();
  const isSyncing = useBoolean();
  const isBackingUp = useBoolean();
  const isRedeeming = useBoolean();
  const confirmRedeem = useBoolean();
  const [toAddress, setToAddress] = useState('');
  const [feeRate, setFeeRate] = useState<number | ''>('');
  const [redeemTxid, setRedeemTxid] = useState<string>('');
  const popover = usePopover();

  const handleAddressChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setToAddress(event.target.value);
  };

  const handleFeeRateChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setFeeRate(Number(event.target.value));
  };

  const handleSync = useCallback(async () => {
    isSyncing.onTrue();

    try {
      await sync();

      toast.success(t('node_current_balance.sync_success'));
      mutate(endpointKeys.lightning.node.info);
      popover.onClose();
    } catch (error) {
      toast.error(error.reason);
    } finally {
      isSyncing.onFalse();
    }
  }, [popover, isSyncing, t]);

  const handleBackup = useCallback(async () => {
    isBackingUp.onTrue();

    try {
      const { data } = await backup({ parseAs: 'blob' });

      const url = window.URL.createObjectURL(data as Blob);

      const a = document.createElement('a');
      a.href = url;
      a.download = `${id.substring(0, 8)}_channels_backup.txt`;
      document.body.appendChild(a);
      a.click();
      window.URL.revokeObjectURL(url);
      document.body.removeChild(a);
    } catch (error) {
      toast.error(error.reason);
    } finally {
      isBackingUp.onFalse();
    }
  }, [isBackingUp, id]);

  const handleRedeem = useCallback(async () => {
    isRedeeming.onTrue();

    if (typeof feeRate === 'number' && feeRate > 0) {
      try {
        const { data } = await redeem({ body: { to_address: toAddress, feerate: feeRate } });

        setRedeemTxid(data!.txid);
        mutate(endpointKeys.lightning.node.info);
      } catch (error) {
        toast.error(error.reason);
      } finally {
        isRedeeming.onFalse();
      }
    } else {
      toast.error('Invalid fee rate');
    }
  }, [toAddress, feeRate, isRedeeming]);

  return (
    <Card sx={{ p: 3, ...sx }} {...other}>
      <IconButton
        color="inherit"
        onClick={popover.onOpen}
        sx={{
          top: 8,
          right: 8,
          zIndex: 9,
          opacity: 0.7,
          position: 'absolute',
          ...(popover.open && { opacity: 1 }),
        }}
      >
        <Iconify icon="eva:more-vertical-fill" />
      </IconButton>

      <Typography variant="subtitle2" gutterBottom>
        {title}
      </Typography>

      <Stack spacing={2}>
        <SatsWithIcon amountMSats={max_payable_msat} variant="h3" />

        <Stack direction="row" justifyContent="space-between">
          <Typography variant="body2" sx={{ color: 'text.secondary' }}>
            {t('node_current_balance.max_receivable')}
          </Typography>
          <SatsWithIcon amountMSats={max_receivable_msat} />
        </Stack>

        <Stack direction="row" justifyContent="space-between">
          <Typography variant="body2" sx={{ color: 'text.secondary' }}>
            {t('node_current_balance.channels_balance')}
          </Typography>
          <SatsWithIcon amountMSats={channels_balance_msat} />
        </Stack>

        <Stack direction="row" justifyContent="space-between">
          <Typography variant="body2" sx={{ color: 'text.secondary' }}>
            {t('node_current_balance.inbound_liquidity')}
          </Typography>
          <SatsWithIcon amountMSats={inbound_liquidity_msats} />
        </Stack>

        <Stack direction="row" justifyContent="space-between">
          <Typography variant="body2" sx={{ color: 'text.secondary' }}>
            {t('node_current_balance.onchain_balance')}
          </Typography>
          <SatsWithIcon amountMSats={onchain_balance_msat} />
        </Stack>

        {pending_onchain_balance_msat > 0 && (
          <Stack direction="row" justifyContent="space-between">
            <Typography variant="body2" sx={{ color: 'text.secondary' }}>
              {t('node_current_balance.pending_onchain_balance')}
            </Typography>
            <SatsWithIcon amountMSats={pending_onchain_balance_msat} />
          </Stack>
        )}
      </Stack>

      <Dialog open={confirmRedeem.value} onClose={confirmRedeem.onFalse}>
        <DialogTitle>{t('node_current_balance.redeem_dialog.title')}</DialogTitle>

        {redeemTxid === '' ? (
          <DialogContent>
            <Typography sx={{ mb: 3 }}>
              {t('node_current_balance.redeem_dialog.description', { amount: fSats(onchain_balance_msat / 1000) })}
            </Typography>

            <TextField
              autoFocus
              fullWidth
              type="text"
              margin="dense"
              variant="outlined"
              label={t('node_current_balance.redeem_dialog.address_label')}
              value={toAddress}
              onChange={handleAddressChange}
            />
            <TextField
              fullWidth
              type="number"
              margin="dense"
              variant="outlined"
              label={t('node_current_balance.redeem_dialog.fee_rate_label')}
              value={feeRate}
              onChange={handleFeeRateChange}
            />
          </DialogContent>
        ) : (
          <DialogContent sx={{ color: 'text.secondary' }}>
            {t('node_current_balance.redeem_dialog.transaction_id')}
            <br />
            <Typography variant="body2">
              <Link fontWeight="bold" href={`https://mempool.space/tx/${redeemTxid}`} target="_blank">
                - {redeemTxid}
              </Link>
            </Typography>
          </DialogContent>
        )}

        <DialogActions>
          {redeemTxid === '' ? (
            <>
              <Button onClick={confirmRedeem.onFalse} variant="outlined" color="inherit">
                {t('cancel')}
              </Button>
              <Button onClick={handleRedeem} variant="contained" disabled={!toAddress || !feeRate || isRedeeming.value}>
                {t('node_current_balance.redeem_dialog.redeem_button')}
              </Button>
            </>
          ) : (
            <Button onClick={confirmRedeem.onFalse} variant="contained">
              {t('close')}
            </Button>
          )}
        </DialogActions>
      </Dialog>

      <CustomPopover open={popover.open} anchorEl={popover.anchorEl} onClose={popover.onClose}>
        <MenuList>
          <MenuItem onClick={handleSync} disabled={isSyncing.value}>
            <Iconify icon="eva:sync-fill" />
            {t('node_current_balance.sync_node')}
          </MenuItem>
        </MenuList>

        <MenuList>
          <MenuItem onClick={handleBackup} disabled={isBackingUp.value}>
            <Iconify icon="eva:save-fill" />
            {t('node_current_balance.download_backup')}
          </MenuItem>
        </MenuList>

        {onchain_balance_msat > 0 && (
          <MenuList>
            <MenuItem onClick={confirmRedeem.onTrue} disabled={confirmRedeem.value}>
              <Iconify icon="eva:arrow-circle-down-fill" />
              {t('node_current_balance.redeem_onchain_funds')}
            </MenuItem>
          </MenuList>
        )}
      </CustomPopover>
    </Card>
  );
}
