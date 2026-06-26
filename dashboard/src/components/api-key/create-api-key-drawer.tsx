import type { DrawerProps } from '@mui/material/Drawer';

import Link from '@mui/material/Link';
import Stack from '@mui/material/Stack';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { Iconify } from 'src/components/iconify';

import { CreateApiKeyForm } from './create-api-key-form';

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
};

export function CreateApiKeyDrawer({ title, isAdmin, open, onClose, onSuccess }: Props) {
  return (
    <Drawer anchor="right" open={open} onClose={onClose} slotProps={{ paper: { sx: drawerSx } }}>
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
      >
        <Typography variant="h6">{title || 'Create API key'}</Typography>
        <IconButton onClick={onClose}>
          <Iconify icon="mingcute:close-line" />
        </IconButton>
      </Stack>

      <Divider />

      <Stack spacing={3} sx={{ p: 3 }}>
        <Typography variant="body2" sx={{ color: 'text.secondary' }}>
          Read our{' '}
          <Link href="https://docs.numeraire.tech/developers/api-keys" target="_blank">
            documentation
          </Link>{' '}
          for information on API tokens.
        </Typography>

        <CreateApiKeyForm isAdmin={isAdmin} onSuccess={onSuccess} />
      </Stack>
    </Drawer>
  );
}
