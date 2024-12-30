import type { Contact } from 'src/lib/swissknife';
import type { IFiatPrices } from 'src/types/bitcoin';
import type { DialogProps } from '@mui/material/Dialog';
import type { IDetectedBarcode } from '@yudiel/react-qr-scanner';

import { decode } from 'light-bolt11-decoder';
import { useState, useCallback } from 'react';
import { varAlpha } from 'minimal-shared/utils';
import { useBoolean } from 'minimal-shared/hooks';
import { Scanner } from '@yudiel/react-qr-scanner';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Avatar from '@mui/material/Avatar';
import Dialog from '@mui/material/Dialog';
import Tooltip from '@mui/material/Tooltip';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import DialogActions from '@mui/material/DialogActions';

import { paths } from 'src/routes/paths';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { Carousel, useCarousel, CarouselArrowFloatButtons } from 'src/components/carousel';

import { ConfirmPaymentDialog } from './confirm-payment-dialog';

// ----------------------------------------------------------------------

export type NewPaymentFormProps = {
  contacts?: Contact[];
  balance?: number;
  fiatPrices: IFiatPrices;
  onSuccess?: () => void;
  isAdmin?: boolean;
  walletId?: string;
};

export function NewPaymentForm({
  balance,
  fiatPrices,
  isAdmin,
  walletId,
  contacts,
  onSuccess,
}: NewPaymentFormProps) {
  const { t } = useTranslate();
  const [input, setInput] = useState('');
  const [bolt11, setBolt11] = useState(undefined);
  const confirm = useBoolean();
  const scanQR = useBoolean();

  const carousel = useCarousel({
    loop: true,
    dragFree: true,
    slidesToShow: 'auto',
    slideSpacing: '20px',
  });

  const handleChangeInput = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    setInput(event.target.value);
  }, []);

  const handleConfirm = useCallback(
    (event: any) => {
      event.preventDefault();
      try {
        const decodedBolt11 = decode(input);
        setBolt11(decodedBolt11);
      } catch {
        setBolt11(undefined);
      }
      confirm.onTrue();
    },
    [input, setBolt11, confirm]
  );

  const handlerClickDot = useCallback(
    (index: number) => {
      if (contacts === undefined) return;

      carousel.dots.onClickDot(index);
      setInput(contacts[index].ln_address);
    },
    [contacts, carousel.dots]
  );

  const handleClose = () => {
    setInput('');
    setBolt11(undefined);
    confirm.onFalse();
  };

  return (
    <>
      {contacts && contacts.length > 0 && (
        <>
          <Stack direction="row" alignItems="center" justifyContent="space-between">
            <Typography variant="overline" sx={{ color: 'text.secondary' }}>
              {t('new_payment.recent')}
            </Typography>

            <Button
              href={paths.wallet.contacts}
              size="small"
              color="inherit"
              endIcon={<Iconify icon="eva:arrow-ios-forward-fill" width={18} sx={{ ml: -0.5 }} />}
              sx={{ mr: -1 }}
            >
              {t('view_all')}
            </Button>
          </Stack>

          <Box sx={{ position: 'relative' }}>
            <CarouselArrowFloatButtons
              {...carousel.arrows}
              options={carousel.options}
              slotProps={{ prevBtn: { svgSize: 14 }, nextBtn: { svgSize: 14 } }}
              sx={[
                (theme) => ({
                  p: 0.5,
                  borderRadius: '50%',
                  bgcolor: varAlpha(theme.vars.palette.text.primaryChannel, 0.48),
                  '&:hover': { bgcolor: theme.vars.palette.text.primary },
                }),
              ]}
            />

            <Carousel carousel={carousel} sx={{ py: 5 }}>
              {contacts.map((contact, index) => (
                <Tooltip key={contact.ln_address} title={contact.ln_address} arrow placement="top">
                  <Avatar
                    alt={contact.ln_address}
                    onClick={() => handlerClickDot(index)}
                    sx={[
                      (theme) => ({
                        mx: 'auto',
                        opacity: 0.48,
                        cursor: 'pointer',
                        transition: theme.transitions.create(['all']),
                        ...(index === carousel.dots.selectedIndex && {
                          opacity: 1,
                          transform: 'scale(1.25)',
                          boxShadow: `-4px 12px 24px 0 ${varAlpha(theme.vars.palette.common.blackChannel, 0.12)}`,
                          ...theme.applyStyles('dark', {
                            boxShadow: `-4px 12px 24px 0 ${varAlpha(theme.vars.palette.common.blackChannel, 0.24)}`,
                          }),
                        }),
                      }),
                    ]}
                  >
                    {contact.ln_address?.charAt(0).toUpperCase()}
                  </Avatar>
                </Tooltip>
              ))}
            </Carousel>
          </Box>
        </>
      )}

      <Stack spacing={3}>
        <TextField
          multiline
          maxRows={4}
          variant="outlined"
          fullWidth
          name="input"
          label={t('new_payment.recipient')}
          placeholder={t('new_payment.recipient_placeholder')}
          onChange={handleChangeInput}
          value={input}
        />

        {balance != null && (
          <Stack direction="row" alignItems="center" sx={{ typography: 'subtitle2' }}>
            <Box component="span" sx={{ flexGrow: 1 }}>
              {t('new_payment.your_balance')}{' '}
            </Box>
            <SatsWithIcon amountMSats={balance} />
          </Stack>
        )}

        <Stack direction="row" spacing={2}>
          <Button
            size="large"
            color="inherit"
            variant="contained"
            disabled={input.length < 5}
            onClick={handleConfirm}
            sx={{ flex: 1 }}
          >
            {t('new_payment.transfer')}
            <Iconify width={16} icon="eva:flash-fill" sx={{ color: '#FF9900', ml: 0.5 }} />
          </Button>

          <Button
            size="large"
            color="inherit"
            variant="contained"
            onClick={scanQR.onTrue}
            sx={{ flex: 1 }}
          >
            {t('new_payment.scan_qr')}{' '}
            <Iconify width={16} icon="solar:qr-code-bold" sx={{ ml: 0.5 }} />
          </Button>
        </Stack>
      </Stack>

      <ScanQRDialog open={scanQR.value} onClose={scanQR.onFalse} onResult={setInput} />
      <ConfirmPaymentDialog
        input={input}
        open={confirm.value}
        onClose={handleClose}
        onSuccess={onSuccess}
        fiatPrices={fiatPrices}
        bolt11={bolt11}
        isAdmin={isAdmin}
        walletId={walletId}
      />
    </>
  );
}

// ----------------------------------------------------------------------

type ScanQRDialogProps = DialogProps & {
  onClose: () => void;
  onResult: (result: string) => void;
};

function ScanQRDialog({ open, onClose, onResult }: ScanQRDialogProps) {
  const { t } = useTranslate();

  const handleScannerResult = (detectedCodes: IDetectedBarcode[]) => {
    const text = detectedCodes[0].rawValue;
    onResult(text);
    onClose();
  };

  return (
    <Dialog open={open} fullWidth maxWidth="xs" onClose={onClose}>
      <Scanner paused={!open} onScan={handleScannerResult} formats={['qr_code']} />

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
