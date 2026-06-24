import type { ReactNode } from 'react';
import type { LabelColor } from 'src/components/label';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';

import { fDate, fTime } from 'src/utils/format-time';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';

// ----------------------------------------------------------------------

export function statusColor(status: string): LabelColor {
  if (status === 'Settled') return 'success';
  if (status === 'Pending') return 'warning';
  if (status === 'Failed' || status === 'Expired') return 'error';
  return 'default';
}

export function ledgerColor(ledger: string): LabelColor {
  if (ledger === 'Lightning') return 'secondary';
  if (ledger === 'Internal') return 'primary';
  if (ledger === 'Onchain') return 'warning';
  return 'default';
}

export function formatDateTime(value?: string | Date | null) {
  if (!value) return 'N/A';

  return `${fDate(value)} ${fTime(value)}`;
}

export function MetricTile({
  title,
  amountMSats,
  helper,
}: {
  title: string;
  amountMSats: number;
  helper?: string;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: 2,
          height: 1,
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
        }),
      ]}
    >
      <Stack spacing={0.75}>
        <Typography variant="caption" color="text.secondary">
          {title}
        </Typography>
        <SatsWithIcon amountMSats={amountMSats} variant="subtitle1" />
        {helper && (
          <Typography variant="caption" color="text.disabled">
            {helper}
          </Typography>
        )}
      </Stack>
    </Box>
  );
}

export function DetailCard({
  title,
  icon,
  children,
}: {
  title: string;
  icon: string;
  children: ReactNode;
}) {
  return (
    <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
      <Stack spacing={2.5}>
        <Stack direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
          <Box
            sx={{
              width: 36,
              height: 36,
              display: 'grid',
              borderRadius: 1,
              placeItems: 'center',
              color: 'primary.main',
              bgcolor: 'primary.lighter',
            }}
          >
            <Iconify icon={icon} width={22} />
          </Box>
          <Typography variant="h6">{title}</Typography>
        </Stack>

        <Stack spacing={2} divider={<Divider flexItem sx={{ borderStyle: 'dashed' }} />}>
          {children}
        </Stack>
      </Stack>
    </Card>
  );
}

export function DetailRow({
  label,
  value,
  copyValue,
  mono,
}: {
  label: string;
  value?: ReactNode;
  copyValue?: string;
  mono?: boolean;
}) {
  return (
    <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5} sx={{ alignItems: { sm: 'center' } }}>
      <Typography variant="body2" color="text.secondary" sx={{ width: { sm: 180 }, flexShrink: 0 }}>
        {label}
      </Typography>
      <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', minWidth: 0, flex: 1 }}>
        <Typography
          variant="body2"
          sx={{
            minWidth: 0,
            wordBreak: 'break-word',
            typography: mono ? 'caption' : 'body2',
            fontFamily: mono ? 'monospace' : undefined,
          }}
        >
          {value || 'N/A'}
        </Typography>
        {copyValue && <CopyButton value={copyValue} title="Copy" />}
      </Stack>
    </Stack>
  );
}

export function TransactionTimeline({
  items,
}: {
  items: Array<{ label: string; value?: string | Date | null; state?: 'done' | 'waiting' | 'error' }>;
}) {
  return (
    <Stack spacing={1.5}>
      {items.map((item) => (
        <Stack key={item.label} direction="row" spacing={1.5} sx={{ alignItems: 'flex-start' }}>
          <Box
            sx={{
              mt: 0.25,
              width: 10,
              height: 10,
              borderRadius: '50%',
              flexShrink: 0,
              bgcolor:
                (item.state === 'done' && 'success.main') ||
                (item.state === 'error' && 'error.main') ||
                'warning.main',
            }}
          />
          <Stack sx={{ minWidth: 0 }}>
            <Typography variant="body2">{item.label}</Typography>
            <Typography variant="caption" color="text.secondary">
              {formatDateTime(item.value)}
            </Typography>
          </Stack>
        </Stack>
      ))}
    </Stack>
  );
}

export function StatusBadges({ status, ledger }: { status: string; ledger: string }) {
  return (
    <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
      <Label variant="soft" color={statusColor(status)}>
        {status}
      </Label>
      <Label variant="soft" color={ledgerColor(ledger)}>
        {ledger}
      </Label>
    </Stack>
  );
}
