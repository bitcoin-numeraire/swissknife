import { useCallback } from 'react';
import { useCopyToClipboard } from 'minimal-shared/hooks';

import { Tooltip, IconButton } from '@mui/material';

import { useTranslate } from 'src/locales';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';

// ----------------------------------------------------------------------

type Props = {
  value?: string;
  title?: string;
};

export default function CopyButton({ value, title }: Props) {
  const { t } = useTranslate();
  const { copy } = useCopyToClipboard();

  const onCopy = useCallback(
    (text?: string) => {
      if (text) {
        copy(text);
        toast.success(t('copied_to_clipboard'));
      }
    },
    [copy, t]
  );

  return (
    <Tooltip title={title}>
      <IconButton onClick={() => onCopy(value)}>
        <Iconify icon="eva:copy-fill" />
      </IconButton>
    </Tooltip>
  );
}
