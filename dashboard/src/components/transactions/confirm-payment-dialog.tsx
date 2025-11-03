import type { IFiatPrices } from 'src/types/bitcoin';
import type { InputProps } from '@mui/material/Input';
import type { DialogProps } from '@mui/material/Dialog';
import type { PaymentResponse, SendPaymentRequest } from 'src/lib/swissknife';

import { m } from 'framer-motion';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
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

import { satsToFiat } from 'src/utils/fiat';
import { fCurrency } from 'src/utils/format-number';
import { handleActionError } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { pay, walletPay } from 'src/lib/swissknife';
import { zSendPaymentRequest } from 'src/lib/swissknife/zod.gen';

import { SatsWithIcon } from 'src/components/bitcoin';
import { RHFTextField } from 'src/components/hook-form';
import { Form } from 'src/components/hook-form/form-provider';
import { varBounce, MotionContainer } from 'src/components/animate';

import { useSettingsContext } from '../settings';
import { WalletSelectDropdown } from '../wallet';

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
  const { state } = useSettingsContext();

  const methods = useForm({
    resolver: zodResolver(zSendPaymentRequest),
    defaultValues: {
      amount_msat: MIN_AMOUNT,
      comment: '',
      wallet: walletId || null,
      input,
    },
  });

  const {
    watch,
    handleSubmit,
    setValue,
    formState: { isSubmitting, isValid },
    reset,
  } = methods;

  const amount = watch('amount_msat');

  const onSubmit = async (body: SendPaymentRequest) => {
    try {
      let paymentResponse;
      const reqBody: SendPaymentRequest = {
        ...body,
        amount_msat: body.amount_msat! * 1000,
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
      handleActionError(error);
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
    if (input.includes(CONFIG.domain)) {
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
                variants={varBounce('in')}
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
                primary={
                  <Typography
                    sx={[
                      (theme) => ({
                        ...theme.mixins.maxLine({ line: 1 }),
                      }),
                    ]}
                  >
                    {truncateText(input, 30)}
                  </Typography>
                }
                secondary={invoiceType()}
              />
            </Stack>

            <Stack direction="row" alignItems="center" spacing={2}>
              <Typography sx={{ flexGrow: 0 }}>
                {fCurrency(satsToFiat(amount!, fiatPrices, state.currency), {
                  currency: state.currency,
                })}
              </Typography>

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
              slotProps={{ htmlInput: { readOnly: !!bolt11 } }}
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
              isAdmin && <WalletSelectDropdown />
            )}
          </Stack>

          <DialogActions>
            <Button onClick={handleClose}>{t('cancel')}</Button>
            <LoadingButton
              type="submit"
              loading={isSubmitting}
              variant="contained"
              disabled={!isValid}
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

function InputAmount({
  autoWidth,
  amount,
  disabled,
  onBlur,
  onChange,
  sx,
  ...other
}: InputAmountProps) {
  return (
    <Stack direction="row" justifyContent="center" spacing={1} sx={sx}>
      <Typography variant="h5">₿</Typography>

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
