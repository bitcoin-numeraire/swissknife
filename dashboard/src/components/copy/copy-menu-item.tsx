import { useCallback } from 'react';

import { MenuItem } from '@mui/material';

import { useCopyToClipboard } from 'src/hooks/use-copy-to-clipboard';

import { useTranslate } from 'src/locales';

import { toast } from 'src/components/snackbar';

import { Iconify } from '../iconify';

// ----------------------------------------------------------------------

interface Props {
  value: string;
  title?: string;
}

export default function CopyMenuItem({ value, title }: Props) {
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
    <MenuItem onClick={() => onCopy(value)}>
      <Iconify icon="eva:copy-fill" />
      {title || t('copy')}
    </MenuItem>
  );
}
