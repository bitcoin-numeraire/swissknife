import type { CardProps } from '@mui/material/Card';
import type { CheckMessageRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import CardHeader from '@mui/material/CardHeader';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { checkMessage } from 'src/lib/swissknife';
import { zCheckMessageRequest } from 'src/lib/swissknife/zod.gen';

import { toast } from 'src/components/snackbar';
import { Form, RHFTextField } from 'src/components/hook-form';

// ----------------------------------------------------------------------

export function VerifyMessage({ ...other }: CardProps) {
  const { t } = useTranslate();

  const methods = useForm({
    resolver: zodResolver(zCheckMessageRequest),
    defaultValues: {
      message: '',
      signature: '',
      pubkey: '',
    },
  });

  const {
    reset,
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const message = watch('message');
  const pubkey = watch('pubkey');
  const signature = watch('signature');

  const onSubmit = async (body: CheckMessageRequest) => {
    try {
      const { data } = await checkMessage({ body });

      if (data!.is_valid) {
        toast.success(t('verify_message.verification_success'));
      } else {
        toast.error(t('verify_message.verification_failed'));
      }

      reset();
    } catch (error) {
      handleActionError(error);
    }
  };

  return (
    <Card {...other}>
      <CardHeader title={t('verify_message.title')} />

      <Stack spacing={3} sx={{ p: 3 }}>
        <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
          <Stack spacing={3}>
            <RHFTextField
              variant="outlined"
              fullWidth
              name="message"
              multiline
              rows={5}
              label={t('verify_message.message_label')}
            />
            <RHFTextField
              variant="outlined"
              fullWidth
              name="signature"
              multiline
              rows={2}
              label={t('verify_message.signature_label')}
            />
            <RHFTextField
              variant="outlined"
              fullWidth
              name="pubkey"
              label={t('verify_message.pubkey_label')}
            />

            <Stack direction="row" spacing={2}>
              <LoadingButton
                type="submit"
                variant="contained"
                color="inherit"
                size="large"
                loading={isSubmitting}
                disabled={!message || !signature || !pubkey || isSubmitting}
                sx={{ flex: 1 }}
              >
                {t('verify_message.verify_button')}
              </LoadingButton>
            </Stack>
          </Stack>
        </Form>
      </Stack>
    </Card>
  );
}
