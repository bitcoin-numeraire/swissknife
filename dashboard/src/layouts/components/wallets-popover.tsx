'use client';

import type { Theme, SxProps } from '@mui/material/styles';
import type { ButtonBaseProps } from '@mui/material/ButtonBase';
import type { Wallet } from 'src/lib/swissknife';

import { useState } from 'react';
import { usePopover } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Avatar from '@mui/material/Avatar';
import MenuList from '@mui/material/MenuList';
import MenuItem from '@mui/material/MenuItem';
import ButtonBase from '@mui/material/ButtonBase';
import ListItemText from '@mui/material/ListItemText';
import CircularProgress from '@mui/material/CircularProgress';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

function walletName(wallet?: Wallet) {
  if (!wallet) return '';
  return wallet.label || wallet.asset?.name || wallet.asset?.display_ticker || 'Wallet';
}

function walletNetwork(wallet?: Wallet) {
  if (wallet?.asset?.network === 'Bitcoin') return 'Mainnet';
  return wallet?.asset?.network || '';
}

function walletTicker(wallet?: Wallet) {
  return wallet?.asset?.display_ticker || wallet?.asset?.code || 'BTC';
}

export function WalletsPopover({ sx, ...other }: ButtonBaseProps) {
  const mediaQuery = 'sm';
  const { t } = useTranslate();
  const { open, anchorEl, onClose, onOpen } = usePopover();
  const { wallets, activeWalletId, walletsLoading, walletsError, selectWallet } =
    useAccountContext();
  const [switchingWalletId, setSwitchingWalletId] = useState<string>();
  const activeWallet = wallets.find((wallet) => wallet.id === activeWalletId);
  const bitcoinLogo = `${CONFIG.assetsDir}/assets/icons/bitcoin/ic-bitcoin.svg`;

  const handleChangeWallet = async (walletId: string) => {
    if (walletId === activeWalletId) {
      onClose();
      return;
    }

    try {
      setSwitchingWalletId(walletId);
      await selectWallet(walletId);
      onClose();
    } catch (error) {
      handleActionError(error);
    } finally {
      setSwitchingWalletId(undefined);
    }
  };

  const buttonBg: SxProps<Theme> = {
    height: 1,
    zIndex: -1,
    opacity: 0,
    content: "''",
    borderRadius: 1,
    position: 'absolute',
    visibility: 'hidden',
    bgcolor: 'action.hover',
    width: 'calc(100% + 8px)',
    transition: (theme) =>
      theme.transitions.create(['opacity', 'visibility'], {
        easing: theme.transitions.easing.sharp,
        duration: theme.transitions.duration.shorter,
      }),
    ...(open && { opacity: 1, visibility: 'visible' }),
  };

  const displayName = walletsLoading
    ? t('wallet_switcher.loading')
    : walletName(activeWallet) || t('wallet_switcher.empty');

  return (
    <>
      <ButtonBase
        disableRipple
        disabled={walletsLoading || !!walletsError || wallets.length === 0}
        onClick={onOpen}
        aria-label={t('wallet_switcher.select')}
        sx={[
          {
            py: 0.5,
            gap: { xs: 0.5, [mediaQuery]: 1 },
            '&::before': buttonBg,
          },
          ...(Array.isArray(sx) ? sx : [sx]),
        ]}
        {...other}
      >
        {walletsLoading ? (
          <CircularProgress size={24} color="inherit" />
        ) : (
          <Box
            component="img"
            alt="Bitcoin"
            src={bitcoinLogo}
            sx={{ width: 24, height: 24, borderRadius: '50%' }}
          />
        )}

        <Box
          component="span"
          sx={{ typography: 'subtitle2', display: { xs: 'none', [mediaQuery]: 'inline-flex' } }}
        >
          {displayName}
        </Box>

        {activeWallet && (
          <Label
            color="info"
            sx={{
              height: 22,
              cursor: 'inherit',
              display: { xs: 'none', [mediaQuery]: 'inline-flex' },
            }}
          >
            {walletNetwork(activeWallet)}
          </Label>
        )}

        <Iconify width={16} icon="carbon:chevron-sort" sx={{ color: 'text.disabled' }} />
      </ButtonBase>

      <CustomPopover
        open={open}
        anchorEl={anchorEl}
        onClose={onClose}
        slotProps={{
          arrow: { placement: 'top-left' },
          paper: { sx: { mt: 0.5, ml: -1.55 } },
        }}
      >
        <MenuList sx={{ width: 280 }}>
          {wallets.map((wallet) => (
            <MenuItem
              key={wallet.id}
              selected={wallet.id === activeWalletId}
              disabled={!!switchingWalletId}
              onClick={() => handleChangeWallet(wallet.id)}
              sx={{ minHeight: 56 }}
            >
              <Avatar alt={walletName(wallet)} src={bitcoinLogo} sx={{ width: 28, height: 28 }} />

              <ListItemText
                primary={walletName(wallet)}
                secondary={wallet.label ? wallet.asset?.name : undefined}
                sx={{ mx: 1.5 }}
              />

              {switchingWalletId === wallet.id ? (
                <CircularProgress size={18} color="inherit" />
              ) : (
                <Label color="info">{walletTicker(wallet)}</Label>
              )}
            </MenuItem>
          ))}
        </MenuList>
      </CustomPopover>
    </>
  );
}
