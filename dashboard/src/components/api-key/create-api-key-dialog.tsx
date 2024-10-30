import type { DialogProps } from '@mui/material/Dialog';

import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';
import { Link, Typography, DialogContent } from '@mui/material';

import { useTranslate } from 'src/locales';

import { CreateApiKeyForm } from './create-api-key-form';

// ----------------------------------------------------------------------

type Props = DialogProps & {
  onClose: VoidFunction;
  title?: string;
  onSuccess: VoidFunction;
  isAdmin?: boolean;
};

export function CreateApiKeyDialog({ title, isAdmin, open, onClose, onSuccess }: Props) {
  const { t } = useTranslate();

  return (
    <Dialog open={open} fullWidth maxWidth="sm" onClose={onClose}>
      <DialogTitle>{title || 'Create API Key'}</DialogTitle>

      <DialogContent sx={{ p: 4 }}>
        <Typography variant="body1" sx={{ color: 'text.secondary', mb: 4 }}>
          Read our{' '}
          <Link href="https://docs.numeraire.tech/developers/api-keys" target="_blank">
            documentation
          </Link>{' '}
          for information on API tokens.
        </Typography>
        <CreateApiKeyForm isAdmin={isAdmin} onSuccess={onSuccess} />
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
