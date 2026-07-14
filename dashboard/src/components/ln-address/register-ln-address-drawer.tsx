import type { DrawerProps } from '@mui/material/Drawer';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { Iconify } from 'src/components/iconify';

import { RegisterLnAddressForm } from './register-ln-address-form';

// ----------------------------------------------------------------------

const drawerSx = {
  width: { xs: 1, sm: 520 },
  maxWidth: 1,
};

type Props = DrawerProps & {
  onClose: VoidFunction;
  title?: string;
  onSuccess: VoidFunction;
  isAdmin?: boolean;
  accountId?: string;
};

export function RegisterLnAddressDrawer({
  title,
  isAdmin,
  open,
  onClose,
  onSuccess,
  accountId,
}: Props) {
  return (
    <Drawer anchor="right" open={open} onClose={onClose} slotProps={{ paper: { sx: drawerSx } }}>
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
      >
        <Typography variant="h6">{title || 'Register Lightning Address'}</Typography>
        <IconButton onClick={onClose}>
          <Iconify icon="mingcute:close-line" />
        </IconButton>
      </Stack>

      <Divider />

      <Box sx={{ p: 3 }}>
        <RegisterLnAddressForm isAdmin={isAdmin} accountId={accountId} onSuccess={onSuccess} />
      </Box>
    </Drawer>
  );
}
