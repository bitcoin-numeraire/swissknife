import type { CardProps } from '@mui/material/Card';
import type { SignMessageRequest } from 'src/lib/swissknife';

import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import CardHeader from '@mui/material/CardHeader';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { signMessage } from 'src/lib/swissknife';
import { zSignMessageRequest } from 'src/lib/swissknife/zod.gen';

import { QRDialog } from 'src/components/qr';
import { Form, RHFTextField } from 'src/components/hook-form';

// ----------------------------------------------------------------------

export function SignMessage({ ...other }: CardProps) {
  const { t } = useTranslate();
  const [qrValue, setQrValue] = useState('');
  const confirm = useBoolean();

  const methods = useForm({
    resolver: zodResolver(zSignMessageRequest),
    defaultValues: {
      message: '',
    },
  });

  const {
    reset,
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const message = watch('message');

  const onSubmit = async (body: SignMessageRequest) => {
    try {
      const { data } = await signMessage({ body });

      setQrValue(data!.signature);
      confirm.onTrue();
      reset();
    } catch (error) {
      handleActionError(error);
    }
  };

  return (
    <Card {...other}>
      <CardHeader title={t('sign_message.title')} />
      <Stack spacing={3} sx={{ p: 3 }}>
        <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
          <Stack spacing={3}>
            <RHFTextField
              variant="outlined"
              fullWidth
              name="message"
              multiline
              rows={5}
              label={t('sign_message.message_label')}
            />

            <Stack direction="row" spacing={2}>
              <LoadingButton
                type="submit"
                variant="contained"
                color="inherit"
                size="large"
                loading={isSubmitting}
                sx={{ flex: 1 }}
                disabled={!message || isSubmitting}
              >
                {t('sign_message.sign_button')}{' '}
              </LoadingButton>
            </Stack>
          </Stack>
        </Form>
      </Stack>

      <QRDialog
        title={t('sign_message.signature_dialog_title')}
        open={confirm.value}
        onClose={confirm.onFalse}
        value={qrValue}
      />
    </Card>
  );
}
