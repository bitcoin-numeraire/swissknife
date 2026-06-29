import type { ReactNode } from 'react';
import type { LabelColor } from 'src/components/label';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Timeline from '@mui/lab/Timeline';
import Tooltip from '@mui/material/Tooltip';
import Divider from '@mui/material/Divider';
import TimelineDot from '@mui/lab/TimelineDot';
import TimelineItem from '@mui/lab/TimelineItem';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import TimelineContent from '@mui/lab/TimelineContent';
import TimelineConnector from '@mui/lab/TimelineConnector';
import TimelineSeparator from '@mui/lab/TimelineSeparator';

import { RouterLink } from 'src/routes/components';

import { fDate, fTime } from 'src/utils/format-time';
import { getLedgerLabel } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';

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

export function TransactionDirectionIcon({ direction }: { direction: 'in' | 'out' }) {
  const isIncoming = direction === 'in';

  return (
    <Box
      sx={{
        width: 52,
        height: 52,
        display: 'grid',
        borderRadius: 1,
        placeItems: 'center',
        color: isIncoming ? 'success.contrastText' : 'warning.contrastText',
        bgcolor: isIncoming ? 'success.main' : 'warning.main',
      }}
    >
      <Iconify
        icon={isIncoming ? 'eva:diagonal-arrow-left-down-fill' : 'eva:diagonal-arrow-right-up-fill'}
        width={30}
      />
    </Box>
  );
}

export function MetricTile({
  title,
  amountMSats,
  value,
  helper,
}: {
  title: string;
  amountMSats?: number;
  value?: ReactNode;
  helper?: string;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: { xs: 1.25, sm: 2 },
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
        {value ? (
          <Typography variant="subtitle1">{value}</Typography>
        ) : (
          <SatsWithIcon amountMSats={amountMSats ?? 0} variant="subtitle1" />
        )}
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
  color = 'primary',
  children,
}: {
  title: string;
  icon: string;
  color?: 'primary' | 'success' | 'warning' | 'info' | 'error';
  children: ReactNode;
}) {
  return (
    <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
      <Stack spacing={2.5}>
        <Stack direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
          <Box
            sx={(theme) => ({
              width: 36,
              height: 36,
              display: 'grid',
              borderRadius: 1,
              placeItems: 'center',
              color: theme.vars.palette[color].main,
              bgcolor: 'background.paper',
              border: `1px solid ${theme.vars.palette[color].main}`,
            })}
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
  href,
  hrefLabel,
  targetBlank = true,
  mono,
}: {
  label: string;
  value?: ReactNode;
  copyValue?: string;
  href?: string;
  hrefLabel?: string;
  targetBlank?: boolean;
  mono?: boolean;
}) {
  return (
    <Stack
      direction={{ xs: 'column', sm: 'row' }}
      spacing={1.5}
      sx={{ alignItems: { sm: 'center' } }}
    >
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
        {href && (
          <Tooltip title={hrefLabel || 'Open'}>
            <IconButton
              component={targetBlank ? 'a' : RouterLink}
              href={href}
              target={targetBlank ? '_blank' : undefined}
              rel={targetBlank ? 'noopener noreferrer' : undefined}
              size="small"
            >
              <Iconify icon="solar:map-arrow-right-bold" width={18} />
            </IconButton>
          </Tooltip>
        )}
      </Stack>
    </Stack>
  );
}

export function TransactionTimeline({
  items,
}: {
  items: Array<{
    label: string;
    value?: string | Date | null;
    state?: 'done' | 'waiting' | 'error';
  }>;
}) {
  return (
    <Timeline
      sx={{
        m: 0,
        p: 0,
        [`& .MuiTimelineItem-root:before`]: { display: 'none' },
      }}
    >
      {items.map((item, index) => (
        <TimelineItem key={item.label}>
          <TimelineSeparator>
            <TimelineDot
              color={
                item.state === 'done' ? 'success' : item.state === 'error' ? 'error' : 'warning'
              }
              sx={{ my: 0.5 }}
            />
            {index < items.length - 1 && <TimelineConnector />}
          </TimelineSeparator>
          <TimelineContent sx={{ pt: 0, pb: 2, px: 2 }}>
            <Typography variant="body2">{item.label}</Typography>
            <Typography variant="caption" color="text.secondary">
              {formatDateTime(item.value)}
            </Typography>
          </TimelineContent>
        </TimelineItem>
      ))}
    </Timeline>
  );
}

export function StatusBadges({ status, ledger }: { status: string; ledger: string }) {
  const { t } = useTranslate();

  return (
    <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
      <Label variant="soft" color={statusColor(status)}>
        {status}
      </Label>
      <Label variant="soft" color={ledgerColor(ledger)}>
        {getLedgerLabel(ledger, t)}
      </Label>
    </Stack>
  );
}
