import type { DialogProps } from '@mui/material/Dialog';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';

import { useTranslate } from 'src/locales';

import { NewInvoiceForm } from './new-invoice-form';

import type { NewInvoiceFormProps } from './new-invoice-form';

// ----------------------------------------------------------------------

type Props = DialogProps &
  NewInvoiceFormProps & {
    onClose: VoidFunction;
  };

export function NewInvoiceDialog({ title, open, onClose, ...other }: Props) {
  const { t } = useTranslate();

  return (
    <Dialog open={open} fullWidth maxWidth="xs" onClose={onClose}>
      <DialogTitle>{title || t('new_invoice.generate_invoice')}</DialogTitle>

      <Box sx={{ p: 4 }}>
        <NewInvoiceForm {...other} />
      </Box>

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
