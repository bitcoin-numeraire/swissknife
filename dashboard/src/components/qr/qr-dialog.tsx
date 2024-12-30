import type { DialogProps } from '@mui/material/Dialog';

import { useCallback } from 'react';
import { QRCode } from 'react-qrcode-logo';
import { useCopyToClipboard } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';

interface QRDialogProps extends DialogProps {
  value: string;
  onClose: VoidFunction;
}

export function QRDialog({ open, value, title, onClose }: QRDialogProps) {
  const { copy } = useCopyToClipboard();

  const onCopy = useCallback(() => {
    if (value) {
      copy(value);
      toast.success('Copied to clipboard!');
    }
  }, [copy, value]);

  return (
    <Dialog open={open} fullWidth maxWidth="sm" onClose={onClose}>
      <DialogTitle>{title || 'Receive Bitcoin'}</DialogTitle>

      <Box
        sx={{
          p: 3,
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          width: '100%',
        }}
      >
        <Box
          sx={{
            width: '100%',
            maxWidth: 360,
            height: 'auto',
            '& > canvas': {
              width: '100% !important',
              height: 'auto !important',
            },
          }}
        >
          <QRCode
            value={value}
            size={360} // Base size, will be overridden by CSS
            logoImage="/logo/logo_square_negative.svg"
            removeQrCodeBehindLogo
            logoPaddingStyle="circle"
            eyeRadius={5}
            logoPadding={3}
          />
        </Box>
      </Box>

      <DialogActions>
        <Button onClick={onClose}>Close</Button>
        <Button variant="contained" onClick={onCopy}>
          <Iconify width={20} icon="eva:copy-fill" /> Copy
        </Button>
        <Button variant="contained" onClick={onClose}>
          <Iconify width={20} icon="eva:share-fill" /> Share
        </Button>
      </DialogActions>
    </Dialog>
  );
}
