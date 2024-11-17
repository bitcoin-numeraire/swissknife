import type { IFiatPrices } from 'src/types/bitcoin';
import type { InputProps } from '@mui/material/Input';
import type { LnAddress, NewInvoiceRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { ajvResolver } from '@hookform/resolvers/ajv';
import { useState, useEffect, useCallback } from 'react';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Input, { inputClasses } from '@mui/material/Input';

import { useBoolean } from 'src/hooks/use-boolean';

import { ajvOptions } from 'src/utils/ajv';
import { satsToFiat } from 'src/utils/fiat';
import { displayLnAddress } from 'src/utils/lnurl';
import { fCurrency } from 'src/utils/format-number';

import { useTranslate } from 'src/locales';
import { generateInvoice, newWalletInvoice, NewInvoiceRequestSchema } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Form } from 'src/components/hook-form/form-provider';
import { RHFSlider, RHFTextField, RHFWalletSelect } from 'src/components/hook-form';

import { QRDialog } from '../qr';
import { useSettingsContext } from '../settings';

// ----------------------------------------------------------------------

const MIN_AMOUNT = 0;
const MAX_AMOUNT = 200000;

// ----------------------------------------------------------------------

export type NewInvoiceFormProps = {
  lnAddress?: LnAddress | null;
  fiatPrices: IFiatPrices;
  onSuccess?: VoidFunction;
  isAdmin?: boolean;
  walletId?: string;
};

// @ts-ignore
const resolver = ajvResolver(NewInvoiceRequestSchema, ajvOptions);

export function NewInvoiceForm({ fiatPrices, isAdmin, walletId, lnAddress, onSuccess }: NewInvoiceFormProps) {
  const { t } = useTranslate();
  const [autoWidth, setAutoWidth] = useState(24);
  const [qrValue, setQrValue] = useState('');
  const confirm = useBoolean();
  const { currency } = useSettingsContext();

  const methods = useForm({
    resolver,
    defaultValues: {
      amount_msat: MIN_AMOUNT,
      description: '',
      wallet: null,
    },
  });

  const {
    watch,
    setValue,
    handleSubmit,
    reset,
    formState: { isSubmitting },
  } = methods;

  const amount = watch('amount_msat');
  const wallet = watch('wallet');

  const handleAutoWidth = useCallback(() => {
    const getNumberLength = amount.toString().length;
    setAutoWidth(getNumberLength * 24);
  }, [amount]);

  useEffect(() => {
    handleAutoWidth();
  }, [amount, handleAutoWidth]);

  const handleChangeSlider = useCallback(
    (_: Event, newValue: number | number[]) => {
      setValue('amount_msat', newValue as number);
    },
    [setValue]
  );

  const handleChangeAmount = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      setValue('amount_msat', Number(event.target.value));
    },
    [setValue]
  );

  const handleBlur = useCallback(() => {
    if (amount < 0) {
      setValue('amount_msat', 0);
    } else if (amount > MAX_AMOUNT) {
      setValue('amount_msat', MAX_AMOUNT);
    }
  }, [amount, setValue]);

  const onSubmit = async (body: any) => {
    try {
      let invoice;
      const reqBody: NewInvoiceRequest = {
        amount_msat: body.amount_msat * 1000,
        description: body.description || undefined,
        wallet_id: walletId || body.wallet?.id,
      };

      if (isAdmin) {
        const { data } = await generateInvoice({ body: reqBody });
        invoice = data!;
      } else {
        const { data } = await newWalletInvoice({ body: reqBody });
        invoice = data!;
      }

      setQrValue(invoice.ln_invoice!.bolt11);
      confirm.onTrue();
      reset();
      onSuccess?.();
    } catch (error) {
      toast.error(error.reason);
    }
  };

  return (
    <>
      <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
        <Stack spacing={3}>
          <Typography variant="overline" sx={{ color: 'text.secondary' }}>
            {t('new_invoice.insert_amount')}
          </Typography>

          <InputAmount amount={amount} onBlur={handleBlur} autoWidth={autoWidth} onChange={handleChangeAmount} />

          <RHFSlider name="amount_msat" min={MIN_AMOUNT} max={MAX_AMOUNT} onChange={handleChangeSlider} onBlur={handleBlur} />

          <Stack direction="row" alignItems="center" sx={{ typography: 'subtitle2' }}>
            <Box component="span" sx={{ flexGrow: 1 }}>
              {t('new_invoice.btc_exchange_rate', { rate: fCurrency(fiatPrices[currency], { currency }) })}
            </Box>
            {fCurrency(satsToFiat(amount, fiatPrices, currency), { currency })}
          </Stack>

          <RHFTextField variant="outlined" fullWidth name="description" label={t('new_invoice.add_note')} />

          {walletId ? (
            <RHFTextField
              variant="outlined"
              value={walletId}
              fullWidth
              name="wallet_id"
              label={t('wallet')}
              inputProps={{ readOnly: true }}
            />
          ) : (
            isAdmin && <RHFWalletSelect />
          )}

          <Stack direction="row" spacing={2}>
            <LoadingButton
              type="submit"
              size="large"
              color="inherit"
              variant="contained"
              disabled={!amount || isSubmitting || (isAdmin && !walletId && !wallet)}
              sx={{ flex: 1 }}
              loading={isSubmitting}
            >
              {t('new_invoice.receive')} <Iconify width={16} icon="eva:flash-fill" sx={{ color: '#FF9900', ml: 0.5 }} />
            </LoadingButton>

            {lnAddress && (
              <Button
                color="inherit"
                variant="contained"
                disabled={!lnAddress.active}
                onClick={() => {
                  setQrValue(displayLnAddress(lnAddress.username));
                  confirm.onTrue();
                }}
                sx={{ flex: 1 }}
              >
                Paycode @
              </Button>
            )}
          </Stack>
        </Stack>
      </Form>
      <QRDialog open={confirm.value} onClose={confirm.onFalse} value={qrValue} />
    </>
  );
}

// ----------------------------------------------------------------------

type InputAmountProps = InputProps & {
  autoWidth: number;
  amount: number | number[];
};

function InputAmount({ autoWidth, amount, onBlur, onChange, sx, ...other }: InputAmountProps) {
  return (
    <Stack direction="row" justifyContent="center" spacing={1} sx={sx}>
      <Typography variant="h5">
        <i className="fak fa-regular" />
      </Typography>

      <Input
        name="amount"
        size="small"
        value={amount}
        onChange={onChange}
        onBlur={onBlur}
        inputProps={{
          min: MIN_AMOUNT,
          max: MAX_AMOUNT,
          type: 'number',
          id: 'input-amount',
        }}
        sx={{
          [`& .${inputClasses.input}`]: {
            p: 0,
            typography: 'h3',
            textAlign: 'center',
            width: autoWidth,
          },
        }}
        {...other}
      />
    </Stack>
  );
}
