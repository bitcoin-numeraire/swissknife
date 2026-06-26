'use client';

import type {
  DashboardModuleDiagnostic,
  DashboardModuleGateReason,
} from '../nav-config-dashboard';

import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Table from '@mui/material/Table';
import TableRow from '@mui/material/TableRow';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableHead from '@mui/material/TableHead';
import Typography from '@mui/material/Typography';
import TableContainer from '@mui/material/TableContainer';

import { Label } from 'src/components/label';

// ----------------------------------------------------------------------

type Props = {
  diagnostics: DashboardModuleDiagnostic[];
  enabledFlags: string[];
  canInspect: boolean;
  mode: string;
};

function formatModes(modes: DashboardModuleDiagnostic['modes']) {
  return modes === 'all' ? 'all' : modes.join(', ');
}

function formatReason(reason: DashboardModuleGateReason) {
  if (reason.type === 'permissions') {
    return `missing ${reason.missing.join(', ')}`;
  }

  if (reason.type === 'mode') {
    return `mode ${reason.activeMode} excluded (${reason.allowedModes.join(', ')})`;
  }

  return `flag ${reason.flag} off`;
}

function renderGateSummary(diagnostic: DashboardModuleDiagnostic) {
  const gates = [
    `requires: ${diagnostic.permissions.length ? diagnostic.permissions.join(', ') : 'none'}`,
    `modes: ${formatModes(diagnostic.modes)}`,
    diagnostic.flag ? `flag: ${diagnostic.flag}` : null,
  ].filter(Boolean);

  return gates.join(' / ');
}

export function ModuleDebugPanel({ diagnostics, enabledFlags, canInspect, mode }: Props) {
  if (!canInspect) {
    return (
      <Alert severity="warning" variant="outlined" sx={{ mx: { xs: 2, md: 5 }, my: 3 }}>
        Module debug is restricted to operators with read:wallet.
      </Alert>
    );
  }

  const visibleCount = diagnostics.filter((diagnostic) => diagnostic.visible).length;
  const hiddenCount = diagnostics.length - visibleCount;

  return (
    <Paper
      variant="outlined"
      sx={{
        mx: { xs: 2, md: 5 },
        my: 3,
        borderRadius: 1,
        bgcolor: 'background.neutral',
      }}
    >
      <Stack
        direction={{ xs: 'column', md: 'row' }}
        spacing={1.5}
        sx={{ p: 2.5, alignItems: { xs: 'flex-start', md: 'center' } }}
      >
        <Box sx={{ flexGrow: 1 }}>
          <Typography variant="subtitle2">Module visibility debug</Typography>
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            Mode: {mode} | Flags: {enabledFlags.length ? enabledFlags.join(', ') : 'none'}
          </Typography>
        </Box>

        <Stack direction="row" spacing={1}>
          <Label color="success">{visibleCount} visible</Label>
          <Label color={hiddenCount ? 'warning' : 'default'}>{hiddenCount} hidden</Label>
        </Stack>
      </Stack>

      <TableContainer sx={{ overflowX: 'auto' }}>
        <Table size="small" sx={{ minWidth: 900 }}>
          <TableHead>
            <TableRow>
              <TableCell>Area</TableCell>
              <TableCell>Module</TableCell>
              <TableCell>Route</TableCell>
              <TableCell>Gates</TableCell>
              <TableCell>Result</TableCell>
            </TableRow>
          </TableHead>

          <TableBody>
            {diagnostics.map((diagnostic) => (
              <TableRow key={`${diagnostic.group}-${diagnostic.title}-${diagnostic.path ?? ''}`}>
                <TableCell sx={{ color: 'text.secondary', whiteSpace: 'nowrap' }}>
                  {diagnostic.group}
                </TableCell>

                <TableCell sx={{ whiteSpace: 'nowrap' }}>
                  <Box sx={{ pl: diagnostic.depth * 2 }}>{diagnostic.title}</Box>
                </TableCell>

                <TableCell sx={{ color: 'text.secondary', maxWidth: 220 }}>
                  <Typography variant="caption" noWrap component="div">
                    {diagnostic.path ?? '-'}
                  </Typography>
                </TableCell>

                <TableCell sx={{ color: 'text.secondary', minWidth: 300 }}>
                  <Typography variant="caption">{renderGateSummary(diagnostic)}</Typography>
                </TableCell>

                <TableCell sx={{ minWidth: 240 }}>
                  <Stack spacing={0.75} sx={{ alignItems: 'flex-start' }}>
                    <Label color={diagnostic.visible ? 'success' : 'warning'}>
                      {diagnostic.visible ? 'visible' : 'hidden'}
                    </Label>

                    {diagnostic.reasons.map((reason) => (
                      <Typography
                        key={formatReason(reason)}
                        variant="caption"
                        sx={{ color: 'text.secondary' }}
                      >
                        {formatReason(reason)}
                      </Typography>
                    ))}
                  </Stack>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
    </Paper>
  );
}
