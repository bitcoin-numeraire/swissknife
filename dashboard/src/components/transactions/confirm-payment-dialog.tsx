import type { IFiatPrices } from 'src/types/bitcoin';
import type { InputProps } from '@mui/material/Input';
import type { DialogProps } from '@mui/material/Dialog';
import type { PaymentResponse, SendPaymentRequest } from 'src/lib/swissknife';

import { m } from 'framer-motion';
import { useForm } from 'react-hook-form';
import { ajvResolver } from '@hookform/resolvers/ajv';
import { useState, useEffect, useCallback } from 'react';

import Box from '@mui/material/Box';
import { Link } from '@mui/material';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import Button from '@mui/material/Button';
import Avatar from '@mui/material/Avatar';
import Dialog from '@mui/material/Dialog';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import DialogTitle from '@mui/material/DialogTitle';
import ListItemText from '@mui/material/ListItemText';
import DialogActions from '@mui/material/DialogActions';
import Input, { inputClasses } from '@mui/material/Input';

import { ajvOptions } from 'src/utils/ajv';
import { satsToFiat } from 'src/utils/fiat';
import { fCurrency } from 'src/utils/format-number';
import { truncateText } from 'src/utils/format-string';

import { maxLine } from 'src/theme/styles';
import { CONFIG } from 'src/config-global';
import { useTranslate } from 'src/locales';
import { pay, walletPay, SendPaymentRequestSchema } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { Form } from 'src/components/hook-form/form-provider';
import { varBounce, MotionContainer } from 'src/components/animate';
import { RHFTextField, RHFWalletSelect } from 'src/components/hook-form';

import { useSettingsContext } from '../settings';

// ----------------------------------------------------------------------

const MIN_AMOUNT = 0;
const MAX_AMOUNT = 200000;

// ----------------------------------------------------------------------

type ConfirmPaymentDialogProps = DialogProps & {
  input: string;
  fiatPrices: IFiatPrices;
  bolt11?: any;
  onClose: () => void;
  onSuccess?: () => void;
  isAdmin?: boolean;
  walletId?: string;
};

// @ts-ignore
const resolver = ajvResolver(SendPaymentRequestSchema, ajvOptions);

export function ConfirmPaymentDialog({
  open,
  input,
  isAdmin,
  walletId,
  fiatPrices,
  bolt11,
  onClose,
  onSuccess,
}: ConfirmPaymentDialogProps) {
  const { t } = useTranslate();

  const [autoWidth, setAutoWidth] = useState(24);
  const [payment, setPayment] = useState<PaymentResponse | undefined>(undefined);
  const { currency } = useSettingsContext();

  const methods = useForm({
    resolver,
    defaultValues: {
      amount_msat: MIN_AMOUNT,
      comment: '',
      wallet: null,
      input,
    },
  });

  const {
    watch,
    handleSubmit,
    setValue,
    formState: { isSubmitting },
    reset,
  } = methods;

  const amount = watch('amount_msat');
  const wallet = watch('wallet');

  const onSubmit = async (body: any) => {
    try {
      let paymentResponse;
      const reqBody: SendPaymentRequest = {
        wallet_id: walletId || body.wallet?.id,
        amount_msat: body.amount_msat! * 1000,
        comment: body.comment || undefined,
        input: body.input,
      };

      if (isAdmin) {
        const { data } = await pay({ body: reqBody });
        paymentResponse = data;
      } else {
        const { data } = await walletPay({ body: reqBody });
        paymentResponse = data;
      }

      reset();
      setPayment(paymentResponse);
      onSuccess?.();
    } catch (error) {
      toast.error(error.reason);
    }
  };

  useEffect(() => {
    if (bolt11) {
      const amountSection = bolt11.sections.find((s: any) => s.name === 'amount');
      const satsAmount = amountSection ? amountSection.value / 1000 : MIN_AMOUNT;
      const comment = bolt11.description || '';

      reset({
        amount_msat: satsAmount,
        comment,
        wallet: null,
        input,
      });
    } else {
      reset({
        amount_msat: MIN_AMOUNT,
        comment: '',
        wallet: null,
        input,
      });
    }
  }, [input, bolt11, reset]);

  const handleAutoWidth = useCallback(() => {
    const getNumberLength = amount.toString().length;
    setAutoWidth(getNumberLength * 24);
  }, [amount]);

  useEffect(() => {
    handleAutoWidth();
  }, [handleAutoWidth, amount]);

  const handleBlur = useCallback(() => {
    if (amount !== undefined) {
      if (amount < 0) {
        setValue('amount_msat', 0);
      } else if (amount > MAX_AMOUNT) {
        setValue('amount_msat', MAX_AMOUNT);
      }
    }
  }, [amount, setValue]);

  const handleChangeAmount = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      setValue('amount_msat', Number(event.target.value));
    },
    [setValue]
  );

  const handleClose = () => {
    reset();
    setPayment(undefined);
    onClose();
  };

  const invoiceType = () => {
    if (bolt11) {
      return t('confirm_payment_dialog.bolt11_transfer');
    }
    if (input.includes(CONFIG.site.domain)) {
      return t('confirm_payment_dialog.internal_transfer');
    }
    if (input.toLowerCase().startsWith('lnurl')) {
      return t('confirm_payment_dialog.lnurl_transfer');
    }
    return t('confirm_payment_dialog.lightning_transfer');
  };

  return (
    <Dialog open={open} fullWidth maxWidth="xs" onClose={handleClose}>
      <DialogTitle>{t('confirm_payment_dialog.title')}</DialogTitle>

      {payment !== undefined ? (
        <>
          <Stack spacing={3} sx={{ px: 3 }} textAlign="center">
            <MotionContainer>
              <Box
                component={m.img}
                src="/assets/icons/payments/success.png"
                alt="payment success"
                variants={varBounce().in}
                maxWidth={250}
                margin="auto"
              />
            </MotionContainer>

            <Stack spacing={3}>
              <SatsWithIcon amountMSats={payment.amount_msat} placement="top" variant="h5">
                {t('confirm_payment_dialog.success_message')} <Link>{truncateText(input, 30)}</Link>
              </SatsWithIcon>

              {payment.success_action?.message && (
                <Tooltip title={t('confirm_payment_dialog.message_from_recipient')} arrow>
                  <Box component={Typography}>
                    <i>{payment.success_action.message}</i>
                  </Box>
                </Tooltip>
              )}
            </Stack>
          </Stack>
          <DialogActions>
            <Button variant="outlined" onClick={handleClose}>
              {t('done')}
            </Button>
          </DialogActions>
        </>
      ) : (
        <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
          <Stack spacing={3} sx={{ px: 3 }}>
            <Stack direction="row" alignItems="center" spacing={2}>
              <Avatar alt={input} sx={{ width: 48, height: 48 }}>
                {input?.charAt(0).toUpperCase()}
              </Avatar>
              <ListItemText
                primary={<Typography sx={{ ...maxLine({ line: 1 }) }}>{truncateText(input, 30)}</Typography>}
                secondary={invoiceType()}
              />
            </Stack>

            <Stack direction="row" alignItems="center" spacing={2}>
              <Typography sx={{ flexGrow: 0 }}>{fCurrency(satsToFiat(amount!, fiatPrices, currency), { currency })}</Typography>

              <InputAmount
                onBlur={handleBlur}
                onChange={handleChangeAmount}
                autoWidth={autoWidth}
                amount={amount!}
                disableUnderline={false}
                sx={{ flexGrow: 1, justifyContent: 'flex-end' }}
                disabled={isSubmitting}
                readOnly={!!bolt11}
              />
            </Stack>

            <RHFTextField
              name="comment"
              fullWidth
              multiline
              rows={3}
              placeholder={t('confirm_payment_dialog.write_message_placeholder')}
              disabled={isSubmitting}
              inputProps={{ readOnly: !!bolt11 }}
            />

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
          </Stack>

          <DialogActions>
            <Button onClick={handleClose}>{t('cancel')}</Button>
            <LoadingButton
              type="submit"
              loading={isSubmitting}
              variant="contained"
              disabled={!amount || isSubmitting || (isAdmin && !walletId && !wallet)}
            >
              {t('confirm_payment_dialog.confirm_send')}
            </LoadingButton>
          </DialogActions>
        </Form>
      )}
    </Dialog>
  );
}

// ----------------------------------------------------------------------

type InputAmountProps = InputProps & {
  autoWidth: number;
  amount: number | number[];
};

function InputAmount({ autoWidth, amount, disabled, onBlur, onChange, sx, ...other }: InputAmountProps) {
  return (
    <Stack direction="row" justifyContent="center" spacing={1} sx={sx}>
      <Typography variant="h5">
        <i className="fak fa-regular" />
      </Typography>

      <Input
        disableUnderline
        size="small"
        value={amount}
        onChange={onChange}
        onBlur={onBlur}
        inputProps={{
          min: MIN_AMOUNT,
          max: MAX_AMOUNT,
          type: 'number',
          id: 'amount_msat',
        }}
        disabled={disabled}
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
