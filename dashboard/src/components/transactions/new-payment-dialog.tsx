import type { DialogProps } from '@mui/material/Dialog';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';

import { useTranslate } from 'src/locales';

import { NewPaymentForm } from './new-payment-form';

import type { NewPaymentFormProps } from './new-payment-form';

// ----------------------------------------------------------------------

type Props = DialogProps &
  NewPaymentFormProps & {
    onClose: VoidFunction;
  };

export function NewPaymentDialog({ title, open, onClose, ...other }: Props) {
  const { t } = useTranslate();

  return (
    <Dialog open={open} fullWidth maxWidth="xs" onClose={onClose}>
      <DialogTitle>{title || t('new_payment.send_payment')}</DialogTitle>

      <Box sx={{ p: 4 }}>
        <NewPaymentForm {...other} />
      </Box>

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
