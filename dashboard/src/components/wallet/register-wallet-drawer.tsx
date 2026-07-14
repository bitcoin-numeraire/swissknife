import type { DrawerProps } from '@mui/material/Drawer';
import type { NewWalletFormProps } from './register-wallet-form';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';

import { RegisterWalletForm } from './register-wallet-form';

// ----------------------------------------------------------------------

const drawerSx = {
  width: { xs: 1, sm: 520 },
  maxWidth: 1,
};

type Props = DrawerProps &
  NewWalletFormProps & {
    onClose: VoidFunction;
  };

export function RegisterWalletDrawer({ title, open, accountId, onClose, onSuccess }: Props) {
  const { t } = useTranslate();

  return (
    <Drawer anchor="right" open={open} onClose={onClose} slotProps={{ paper: { sx: drawerSx } }}>
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
      >
        <Typography variant="h6">{title || t('register_wallet.title')}</Typography>
        <IconButton onClick={onClose}>
          <Iconify icon="mingcute:close-line" />
        </IconButton>
      </Stack>

      <Divider />

      <Box sx={{ p: 3 }}>
        <RegisterWalletForm accountId={accountId} onSuccess={onSuccess} />
      </Box>
    </Drawer>
  );
}
