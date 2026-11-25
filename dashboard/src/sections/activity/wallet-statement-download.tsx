import type { Dayjs } from 'dayjs';

import dayjs from 'dayjs';
import { useState } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import { DatePicker } from '@mui/x-date-pickers/DatePicker';
import {
  Stack,
  Button,
  Dialog,
  Typography,
  DialogTitle,
  DialogActions,
  DialogContent,
} from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { exportWalletStatement, exportAccountWalletStatement } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';

type Props = { walletId: string; admin?: boolean };

export function statementPeriod(first: Dayjs | null, last: Dayjs | null) {
  if (!first?.isValid() || !last?.isValid() || first.isAfter(last, 'day')) return null;
  const midnightUtc = (day: Dayjs) => new Date(day.format('YYYY-MM-DD') + 'T00:00:00Z');
  return { from: midnightUtc(first), until: midnightUtc(last.add(1, 'day')) };
}

export function WalletStatementDownload({ walletId, admin = false }: Props) {
  const { t } = useTranslate();
  const dialog = useBoolean();
  const [first, setFirst] = useState<Dayjs | null>(() =>
    dayjs(new Date().toISOString().slice(0, 10)).startOf('month')
  );
  const [last, setLast] = useState<Dayjs | null>(() =>
    dayjs(new Date().toISOString().slice(0, 10))
  );
  const [downloading, setDownloading] = useState(false);
  const period = statementPeriod(first, last);

  const download = async () => {
    if (!period || downloading) return;
    setDownloading(true);
    try {
      const options = { query: period, parseAs: 'text', throwOnError: true } as const;
      const { data } = admin
        ? await exportWalletStatement({ ...options, path: { id: walletId } })
        : await exportAccountWalletStatement({ ...options, path: { wallet_id: walletId } });
      const blob = new Blob([data], { type: 'text/csv;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `wallet-${walletId}-${first!.format('YYYY-MM-DD')}-${last!.format('YYYY-MM-DD')}.csv`;
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
      dialog.onFalse();
    } catch (error) {
      handleActionError(error);
    } finally {
      setDownloading(false);
    }
  };

  return (
    <>
      <Button
        color="inherit"
        variant="outlined"
        onClick={dialog.onTrue}
        startIcon={<Iconify icon="eva:download-outline" />}
      >
        {t('statement.action')}
      </Button>
      <Dialog
        fullWidth
        maxWidth="xs"
        open={dialog.value}
        onClose={downloading ? undefined : dialog.onFalse}
      >
        <DialogTitle>{t('statement.title')}</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ pt: 1 }}>
            <Typography variant="body2" color="text.secondary">
              {t('statement.wallet', { wallet: walletId })}
            </Typography>
            <DatePicker
              label={t('statement.from')}
              value={first}
              onChange={setFirst}
              disabled={downloading}
            />
            <DatePicker
              label={t('statement.through')}
              value={last}
              onChange={setLast}
              minDate={first ?? undefined}
              disabled={downloading}
              slotProps={{
                textField: {
                  helperText: first && last && !period ? t('end_date_error') : undefined,
                },
              }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button color="inherit" onClick={dialog.onFalse} disabled={downloading}>
            {t('cancel')}
          </Button>
          <Button variant="contained" onClick={download} disabled={!period} loading={downloading}>
            {t('statement.csv')}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
