'use client';

import {
  Box,
  Card,
  Alert,
  Stack,
  Table,
  Button,
  TableRow,
  TableBody,
  TableCell,
  TableHead,
  CardHeader,
  Typography,
  LinearProgress,
  TableContainer,
} from '@mui/material';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useNodeOverview } from 'src/actions/node';
import { DashboardContent } from 'src/layouts/dashboard';

import { RoleBasedGuard } from 'src/auth/guard';

export function NodeHealthView() {
  return (
    <DashboardContent>
      <RoleBasedGuard hasContent permissions={[Permission.READ_LN_NODE]}>
        <NodeObservations />
      </RoleBasedGuard>
    </DashboardContent>
  );
}

function NodeObservations() {
  const { t, currentLang } = useTranslate();
  const { data, error, isLoading, isValidating, mutate } = useNodeOverview();
  const locale = currentLang.numberFormat.code;
  const unknown = t('node_health.unavailable');
  const number = (value?: number | null) =>
    value == null ? unknown : value.toLocaleString(locale);
  const amount = (value?: string | null) =>
    value == null ? unknown : BigInt(value).toLocaleString(locale) + ' msat';
  const balance = (value?: number | null) => (value == null ? unknown : number(value) + ' sat');
  const connectedPeers = data?.peers?.filter((p) => p.connected).length;
  const activeChannels = data?.channels?.filter((c) => c.active).length;

  return (
    <Stack spacing={3}>
      <Stack
        direction="row"
        spacing={2}
        sx={{ justifyContent: 'space-between', alignItems: 'center' }}
      >
        <Box>
          <Typography variant="h4" component="h1">
            {t('node_health.title')}
          </Typography>
          {data && (
            <Typography variant="body2" color="text.secondary">
              {[
                data.info?.alias,
                data.provider,
                data.info?.network,
                data.observed_at.toLocaleString(locale),
              ]
                .filter(Boolean)
                .join(' · ')}
            </Typography>
          )}
        </Box>
        <Button variant="outlined" disabled={isValidating} onClick={() => void mutate()}>
          {t('node_health.refresh')}
        </Button>
      </Stack>
      {isLoading && <LinearProgress aria-label={t('node_health.title')} />}
      {error && <Alert severity="error">{t('node_health.failed')}</Alert>}
      {data && (
        <>
          {data.alerts.map((alert) => (
            <Alert key={alert} severity="warning">
              {t('node_health.alerts.' + alert)}
            </Alert>
          ))}
          <Box
            sx={{
              display: 'grid',
              gap: 2,
              gridTemplateColumns: {
                xs: '1fr',
                sm: 'repeat(2, minmax(0, 1fr))',
                lg: 'repeat(4, minmax(0, 1fr))',
              },
            }}
          >
            {[
              [t('node_health.outbound'), amount(data.liquidity?.outbound_msat)],
              [t('node_health.inbound'), amount(data.liquidity?.inbound_msat)],
              [t('node_health.confirmed'), balance(data.onchain?.confirmed_sat)],
              [t('node_health.unconfirmed'), balance(data.onchain?.unconfirmed_sat)],
            ].map(([label, value]) => (
              <Card key={label} sx={{ p: 3 }}>
                <Typography variant="subtitle2" color="text.secondary">
                  {label}
                </Typography>
                <Typography variant="h5" sx={{ mt: 1, overflowWrap: 'anywhere' }}>
                  {value}
                </Typography>
              </Card>
            ))}
          </Box>
          <Typography variant="body2" color="text.secondary">
            {t(
              data.provider.startsWith('lnd_') ? 'node_health.lnd_basis' : 'node_health.cln_basis'
            )}
          </Typography>
          <Card>
            <CardHeader
              title={t('node_health.channels')}
              subheader={t('node_health.summary', {
                active: number(activeChannels),
                total: number(data.channels?.length),
                peers: number(connectedPeers),
                height: number(data.info?.block_height),
              })}
            />
            {data.channels && data.channels.length > 0 && (
              <TableContainer>
                <Table size="small" aria-label={t('node_health.channels')}>
                  <TableHead>
                    <TableRow>
                      {['peer', 'status', 'outbound', 'inbound'].map((col) => (
                        <TableCell key={col}>{t('node_health.' + col)}</TableCell>
                      ))}
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {data.channels.map((channel, index) => (
                      <TableRow key={channel.id ?? channel.peer_id + '-' + index}>
                        <TableCell sx={{ maxWidth: 340, overflowWrap: 'anywhere' }}>
                          {channel.peer_id}
                          {channel.id && (
                            <Typography
                              variant="caption"
                              sx={{ display: 'block', color: 'text.secondary' }}
                            >
                              {channel.id}
                            </Typography>
                          )}
                        </TableCell>
                        <TableCell>
                          {t(channel.active ? 'node_health.active' : 'node_health.inactive')}
                        </TableCell>
                        <TableCell>{amount(channel.outbound_msat)}</TableCell>
                        <TableCell>{amount(channel.inbound_msat)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            )}
          </Card>
        </>
      )}
    </Stack>
  );
}
