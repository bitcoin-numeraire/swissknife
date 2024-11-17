import type { DialogProps } from '@mui/material/Dialog';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';

import { useTranslate } from 'src/locales';

import { RegisterWalletForm } from './register-wallet-form';

import type { NewWalletFormProps } from './register-wallet-form';

// ----------------------------------------------------------------------

type Props = DialogProps &
  NewWalletFormProps & {
    onClose: VoidFunction;
  };

export function RegisterWalletDialog({ title, open, onClose, onSuccess }: Props) {
  const { t } = useTranslate();

  return (
    <Dialog open={open} fullWidth maxWidth="sm" onClose={onClose}>
      <DialogTitle>{title || t('register_wallet.title')}</DialogTitle>

      <Box sx={{ p: 4 }}>
        <RegisterWalletForm onSuccess={onSuccess} />
      </Box>

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
