'use client';

import useSWR from 'swr';
import { useState } from 'react';

import {
  Box,
  Alert,
  Stack,
  Table,
  TableRow,
  TableBody,
  TableCell,
  TextField,
  Typography,
  LinearProgress,
} from '@mui/material';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { getMerchantRevenue } from 'src/lib/swissknife';

import { Scrollbar } from 'src/components/scrollbar';
import { TableHeadCustom } from 'src/components/table';
import { useSettingsContext } from 'src/components/settings';

function initialRange() {
  const last = new Date();
  const first = new Date(last);
  first.setUTCDate(first.getUTCDate() - 29);
  return { from: first.toISOString().slice(0, 10), through: last.toISOString().slice(0, 10) };
}

export function MerchantRevenue({ walletId }: { walletId: string }) {
  const { t, i18n } = useTranslate();
  const {
    state: { hideBalances },
  } = useSettingsContext();
  const [range, setRange] = useState(initialRange);
  const duration = (Date.parse(range.through) - Date.parse(range.from)) / 86_400_000;
  const valid = duration >= 0 && duration < 366;
  const report = useSWR(
    valid ? endpointKeys.accountWallet.merchantRevenue(walletId, range.from, range.through) : null,
    () =>
      getMerchantRevenue<true>({ path: { wallet_id: walletId }, query: range, throwOnError: true })
  );
  const rows = report.data?.data;
  const numbers = new Intl.NumberFormat(i18n.language);
  const amount = (value: string) => (hideBalances ? '••••' : numbers.format(BigInt(value)));
  return (
    <>
      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ pt: 1, mb: 3 }}>
        {(['from', 'through'] as const).map((field) => (
          <TextField
            key={field}
            type="date"
            size="small"
            label={t(`merchant_revenue.${field}`)}
            value={range[field]}
            onChange={(event) => setRange({ ...range, [field]: event.target.value })}
            slotProps={{ inputLabel: { shrink: true } }}
          />
        ))}
      </Stack>
      {!valid && <Alert severity="warning">{t('merchant_revenue.invalid_range')}</Alert>}
      {valid && report.error && <Alert severity="error">{t('merchant_revenue.load_error')}</Alert>}
      {valid && report.isLoading && <LinearProgress aria-label={t('merchant_revenue.title')} />}
      {valid &&
        !report.error &&
        rows &&
        (rows.length ? (
          <Scrollbar>
            <Table aria-label={t('merchant_revenue.title')} size="small">
              <TableHeadCustom
                headCells={[
                  { id: 'route', label: t('merchant_revenue.route') },
                  { id: 'count', label: t('merchant_revenue.count'), align: 'right' },
                  { id: 'received', label: t('merchant_revenue.received'), align: 'right' },
                  { id: 'fees', label: t('merchant_revenue.fees'), align: 'right' },
                ]}
              />
              <TableBody>
                {rows.map((row) => (
                  <TableRow key={row.route}>
                    <TableCell component="th" scope="row">
                      {row.route}
                    </TableCell>
                    <TableCell align="right">{numbers.format(row.settled_invoices)}</TableCell>
                    <TableCell align="right">{amount(row.received_msat)}</TableCell>
                    <TableCell align="right">{amount(row.fee_msat)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </Scrollbar>
        ) : (
          <Box sx={{ py: 3 }}>
            <Typography color="text.secondary">{t('merchant_revenue.empty')}</Typography>
          </Box>
        ))}
    </>
  );
}
