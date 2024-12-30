import type { BoxProps } from '@mui/material/Box';
import type { IFiatPrices } from 'src/types/bitcoin';
import type { WalletResponse } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import { Typography } from '@mui/material';
import MenuList from '@mui/material/MenuList';
import MenuItem from '@mui/material/MenuItem';
import IconButton from '@mui/material/IconButton';

import { satsToFiat } from 'src/utils/fiat';
import { displayLnAddress } from 'src/utils/lnurl';
import { fCurrency } from 'src/utils/format-number';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';

import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { useSettingsContext } from 'src/components/settings';
import { CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

type Props = BoxProps & {
  wallet: WalletResponse;
  fiatPrices: IFiatPrices;
};

export function CurrentBalance({ wallet, fiatPrices, sx, ...other }: Props) {
  const { t } = useTranslate();
  const { balance, ln_address } = wallet;
  const popover = usePopover();
  const displayAmount = useBoolean();
  const { state } = useSettingsContext();

  return (
    <Box
      sx={{
        borderRadius: 2,
        position: 'relative',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        backgroundRepeat: 'no-repeat',
        backgroundImage: `url('${CONFIG.assetsDir}/assets/background/background-4.jpg')`,
        color: 'common.white',
        ...sx,
      }}
      {...other}
    >
      <Box sx={{ p: 3, width: 1 }}>
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

        <div>
          <Box sx={{ mb: 1.5, typography: 'subtitle2', opacity: 0.7 }}>
            {t('current_balance.current_balance')}
          </Box>
          <Box sx={{ gap: 1, display: 'flex', alignItems: 'center', mb: 3 }}>
            <Box component="span" sx={{ typography: 'h4' }}>
              {displayAmount.value ? (
                '********'
              ) : (
                <SatsWithIcon amountMSats={balance.available_msat} variant="inherit" />
              )}
            </Box>

            <IconButton color="inherit" onClick={displayAmount.onToggle} sx={{ opacity: 0.7 }}>
              <Iconify icon={displayAmount.value ? 'solar:eye-bold' : 'solar:eye-closed-bold'} />
            </IconButton>
          </Box>
        </div>

        {ln_address && (
          <Box
            sx={{
              my: 3,
              gap: 1,
              display: 'flex',
              alignItems: 'center',
              typography: 'subtitle2',
              justifyContent: 'flex-end',
            }}
          >
            <Box
              sx={{
                px: 0.75,
                bgcolor: 'white',
                borderRadius: 0.5,
                display: 'inline-flex',
              }}
            >
              <Iconify width={24} icon="eva:flash-fill" sx={{ color: '#FF9900' }} />
            </Box>
            {displayLnAddress(ln_address.username)}
          </Box>
        )}

        <Box sx={{ gap: 5, display: 'flex', typography: 'body1' }}>
          <div>
            <Box sx={{ mb: 1, opacity: 0.7, typography: 'caption' }}>
              {t('current_balance.fees_paid')}
            </Box>
            <SatsWithIcon amountMSats={balance.fees_paid_msat} />
          </div>
          <div>
            <Box sx={{ mb: 1, opacity: 0.7, typography: 'caption' }}>
              {t('current_balance.fiat_amount')}
            </Box>
            <Typography>
              {fCurrency(satsToFiat(balance.available_msat / 1000, fiatPrices, state.currency), {
                currency: state.currency,
              })}
            </Typography>
          </div>
        </Box>
      </Box>

      <CustomPopover open={popover.open} anchorEl={popover.anchorEl} onClose={popover.onClose}>
        <MenuList>
          <MenuItem
            onClick={() => {
              mutate(endpointKeys.userWallet.get);
              popover.onClose();
            }}
            sx={{ color: 'primary.light' }}
          >
            <Iconify icon="eva:refresh-fill" />
            {t('current_balance.refresh')}
          </MenuItem>
        </MenuList>
      </CustomPopover>
    </Box>
  );
}
