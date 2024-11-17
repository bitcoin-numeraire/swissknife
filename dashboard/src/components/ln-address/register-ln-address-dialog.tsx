import type { DialogProps } from '@mui/material/Dialog';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';

import { useTranslate } from 'src/locales';

import { RegisterLnAddressForm } from './register-ln-address-form';

// ----------------------------------------------------------------------

type Props = DialogProps & {
  onClose: VoidFunction;
  title?: string;
  onSuccess: VoidFunction;
  isAdmin?: boolean;
};

export function RegisterLnAddressDialog({ title, isAdmin, open, onClose, onSuccess }: Props) {
  const { t } = useTranslate();

  return (
    <Dialog open={open} fullWidth maxWidth="sm" onClose={onClose}>
      <DialogTitle>{title || 'Register Lightning Address'}</DialogTitle>

      <Box sx={{ p: 4 }}>
        <RegisterLnAddressForm isAdmin={isAdmin} onSuccess={onSuccess} />
      </Box>

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
