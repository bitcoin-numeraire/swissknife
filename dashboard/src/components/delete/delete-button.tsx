import { LoadingButton } from '@mui/lab';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';

import { useBoolean } from 'src/hooks/use-boolean';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';
import { ConfirmDialog } from 'src/components/custom-dialog';

// ----------------------------------------------------------------------

type Props = {
  id: string;
  onDelete: (id: string) => Promise<void>;
};

export function DeleteButton({ id, onDelete }: Props) {
  const { t } = useTranslate();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  return (
    <>
      <Tooltip title={t('delete')}>
        <IconButton color="error" onClick={confirm.onTrue}>
          <Iconify icon="solar:trash-bin-trash-bold" />
        </IconButton>
      </Tooltip>

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('delete')}
        content={t('confirm_delete')}
        action={
          <LoadingButton
            variant="contained"
            color="error"
            onClick={async () => {
              isDeleting.onTrue();
              await onDelete(id);
              isDeleting.onFalse();
            }}
            loading={isDeleting.value}
          >
            {t('delete')}
          </LoadingButton>
        }
      />
    </>
  );
}
