'use client';

import type { Wallet } from 'src/lib/swissknife';

import { useState } from 'react';

import {
  Box,
  Card,
  Stack,
  Alert,
  Table,
  Button,
  MenuItem,
  TableRow,
  TableHead,
  TableCell,
  TableBody,
  TextField,
  CardHeader,
  CardContent,
  TableContainer,
  LinearProgress,
} from '@mui/material';

import { useTranslate } from 'src/locales';
import { SpendingInterval } from 'src/lib/swissknife';
import { useSpendingReport } from 'src/actions/spending-report';

import { satsFromMsat } from './spending-limits';

const uuid = /^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;

export function SpendingReportCard({
  accountId,
  wallets,
}: {
  accountId: string;
  wallets: Wallet[];
}) {
  const { t, currentLang } = useTranslate();
  const [interval, setInterval] = useState<SpendingInterval>(SpendingInterval.DAY);
  const [wallet, setWallet] = useState('all');
  const [key, setKey] = useState('');
  const [from, setFrom] = useState('');
  const [through, setThrough] = useState('');
  const validKey = !key || uuid.test(key);
  const validRange =
    !from ||
    !through ||
    (from <= through && Date.parse(through) - Date.parse(from) <= 365 * 86400000);
  const query = {
    interval,
    wallet_id: wallet === 'all' ? undefined : wallet,
    agent_key_id: key || undefined,
    from: from || undefined,
    through: through || undefined,
  };
  const { data, error, isLoading } = useSpendingReport(accountId, query, validKey && validRange);
  const formatter = Intl.NumberFormat(currentLang.value, { maximumFractionDigits: 3 });
  const date = Intl.DateTimeFormat(currentLang.value, { dateStyle: 'medium', timeZone: 'UTC' });
  function download(format: 'csv' | 'json') {
    if (!data) return;
    const csv =
      [
        'period_start,payments,charged_msat',
        ...data.map((row) => [row.period_start, row.payments, row.charged_msat].join(',')),
      ].join('\r\n') + '\r\n';
    const blob = new Blob([format === 'csv' ? csv : JSON.stringify(data, null, 2) + '\n'], {
      type: format === 'csv' ? 'text/csv;charset=utf-8' : 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `spending-report-${accountId}.${format}`;
    link.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  return (
    <Card>
      <CardHeader title={t('spending_report.title')} subheader={t('spending_report.description')} />
      <CardContent>
        <Stack spacing={2}>
          <Box
            sx={{
              display: 'grid',
              gridTemplateColumns: {
                xs: '1fr',
                sm: 'repeat(2,minmax(0,1fr))',
                md: 'repeat(3,minmax(0,1fr))',
              },
              gap: 2,
            }}
          >
            <TextField
              select
              label={t('spending_report.interval')}
              value={interval}
              onChange={(e) => setInterval(e.target.value as SpendingInterval)}
            >
              <MenuItem value="day">{t('spending_report.day')}</MenuItem>
              <MenuItem value="week">{t('spending_report.week')}</MenuItem>
            </TextField>
            <TextField
              select
              label={t('spending_report.wallet')}
              value={wallet}
              onChange={(e) => setWallet(e.target.value)}
            >
              <MenuItem value="all">{t('spending_report.all_wallets')}</MenuItem>
              {wallets.map((w) => (
                <MenuItem key={w.id} value={w.id}>
                  {w.label || w.id}
                </MenuItem>
              ))}
            </TextField>
            <TextField
              label={t('spending_report.key')}
              value={key}
              onChange={(e) => setKey(e.target.value.trim())}
              error={!validKey}
              helperText={!validKey && t('spending_report.invalid_key')}
            />
            <TextField
              type="date"
              label={t('spending_report.from')}
              value={from}
              onChange={(e) => setFrom(e.target.value)}
              slotProps={{ inputLabel: { shrink: true } }}
            />
            <TextField
              type="date"
              label={t('spending_report.through')}
              value={through}
              onChange={(e) => setThrough(e.target.value)}
              error={!validRange}
              helperText={!validRange && t('spending_report.invalid_range')}
              slotProps={{ inputLabel: { shrink: true } }}
            />
          </Box>
          {isLoading && <LinearProgress />}
          {error && <Alert severity="error">{t('spending_report.error')}</Alert>}
          {data && validKey && validRange && (
            <>
              {data.length > 0 ? (
                <TableContainer>
                  <Table size="small" aria-label={t('spending_report.title')}>
                    <TableHead>
                      <TableRow>
                        <TableCell>{t('spending_report.period')}</TableCell>
                        <TableCell align="right">{t('spending_report.payments')}</TableCell>
                        <TableCell align="right">{t('spending_report.charged')}</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {data.map((row) => (
                        <TableRow key={row.period_start}>
                          <TableCell>
                            {date.format(new Date(row.period_start + 'T00:00:00Z'))}
                          </TableCell>
                          <TableCell align="right">{row.payments}</TableCell>
                          <TableCell align="right">
                            {formatter.format(satsFromMsat(row.charged_msat) as `${number}`)}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </TableContainer>
              ) : (
                <Alert severity="info">{t('spending_report.empty')}</Alert>
              )}
              <Stack direction="row" spacing={1}>
                <Button onClick={() => download('csv')}>{t('spending_report.csv')}</Button>
                <Button onClick={() => download('json')}>{t('spending_report.json')}</Button>
              </Stack>
            </>
          )}
        </Stack>
      </CardContent>
    </Card>
  );
}
