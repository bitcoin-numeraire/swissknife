'use client';

import useSWR from 'swr';
import { useState } from 'react';

import {
  Box,
  Card,
  Stack,
  Alert,
  Table,
  Button,
  TableRow,
  TableCell,
  TableHead,
  TableBody,
  CardHeader,
  Typography,
  CardContent,
  TableContainer,
  LinearProgress,
} from '@mui/material';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { getAgentSpendingActivity } from 'src/lib/swissknife';

import { Label } from 'src/components/label';

import { satsFromMsat } from './spending-limits';

export function AgentSpendingActivityCard({ accountId }: { accountId: string }) {
  const { t, currentLang } = useTranslate();
  const [cursors, setCursors] = useState<string[]>([]);
  const after = cursors.at(-1);
  const { data, error, isLoading } = useSWR(
    endpointKeys.accounts.agentActivity(accountId, after),
    async () =>
      (
        await getAgentSpendingActivity<true>({
          path: { account_id: accountId },
          query: { after, limit: 10 },
          throwOnError: true,
        })
      ).data,
    { refreshInterval: 30000 }
  );
  const number = Intl.NumberFormat(currentLang.value, { maximumFractionDigits: 3 });
  const time = Intl.DateTimeFormat(currentLang.value, {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  });
  const next = data?.next_after;

  return (
    <Card>
      <CardHeader
        title={t('agent_activity.title')}
        subheader={data && t('agent_activity.updated', { time: time.format(data.as_of) })}
      />
      <CardContent>
        {error && <Alert severity="error">{t('agent_activity.load_error')}</Alert>}
        {isLoading && <LinearProgress aria-label={t('agent_activity.title')} />}
        {data &&
          !error &&
          (data.items.length ? (
            <TableContainer>
              <Table size="small" aria-label={t('agent_activity.title')}>
                <TableHead>
                  <TableRow>
                    <TableCell>{t('agent_activity.credential')}</TableCell>
                    <TableCell align="right">{t('agent_activity.current')}</TableCell>
                    <TableCell align="right">{t('agent_activity.previous')}</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {data.items.map((row) => (
                    <TableRow key={row.agent_key_id}>
                      <TableCell component="th" scope="row" sx={{ overflowWrap: 'anywhere' }}>
                        {row.name && <Typography variant="subtitle2">{row.name}</Typography>}
                        <Typography variant="caption" color="text.secondary">
                          {row.agent_key_id}
                        </Typography>
                      </TableCell>
                      {[row.current_hour, row.previous_hour].map((hour, index) => (
                        <TableCell align="right" key={index === 0 ? 'current' : 'previous'}>
                          <Box sx={{ whiteSpace: 'nowrap' }}>
                            {t('agent_activity.sats', {
                              amount: number.format(satsFromMsat(hour.charged_msat) as `${number}`),
                            })}
                          </Box>
                          <Typography variant="caption" color="text.secondary">
                            {t('agent_activity.payments', { count: hour.payments })}
                          </Typography>
                          {index === 0 && row.above_hourly_warning && (
                            <Box>
                              <Label color="warning" variant="soft">
                                {t('agent_activity.above_warning')}
                              </Label>
                            </Box>
                          )}
                        </TableCell>
                      ))}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          ) : (
            <Typography variant="body2" color="text.secondary">
              {t('agent_activity.empty')}
            </Typography>
          ))}
        {(after || next) && (
          <Stack direction="row" spacing={1} sx={{ justifyContent: 'flex-end', mt: 2 }}>
            <Button disabled={!after} onClick={() => setCursors((pages) => pages.slice(0, -1))}>
              {t('agent_activity.back')}
            </Button>
            <Button
              disabled={!next}
              onClick={() => next && setCursors((pages) => [...pages, next])}
            >
              {t('agent_activity.next')}
            </Button>
          </Stack>
        )}
      </CardContent>
    </Card>
  );
}
