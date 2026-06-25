'use client';

import type { BoxProps } from '@mui/material/Box';
import type { Breakpoint } from '@mui/material/styles';

import { varAlpha } from 'minimal-shared/utils';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';

// ----------------------------------------------------------------------

export type AuthSplitSectionProps = BoxProps & {
  title?: string;
  subtitle?: string;
  layoutQuery?: Breakpoint;
};

const railItems = [
  {
    icon: 'solar:shield-keyhole-bold-duotone',
    title: 'auth_panel.vault_title',
    description: 'auth_panel.vault_description',
    color: 'success.main',
  },
  {
    icon: 'solar:bolt-circle-bold-duotone',
    title: 'auth_panel.rails_title',
    description: 'auth_panel.rails_description',
    color: 'warning.main',
  },
  {
    icon: 'solar:user-id-bold-duotone',
    title: 'auth_panel.identity_title',
    description: 'auth_panel.identity_description',
    color: 'info.main',
  },
];

const statusItems = [
  ['auth_panel.status_node', 'auth_panel.status_node_value'],
  ['auth_panel.status_network', 'auth_panel.status_network_value'],
  ['auth_panel.status_custody', 'auth_panel.status_custody_value'],
];

export function AuthSplitSection({
  sx,
  title,
  subtitle,
  layoutQuery = 'md',
  ...other
}: AuthSplitSectionProps) {
  const { t } = useTranslate();

  return (
    <Box
      sx={[
        (theme) => ({
          px: 3,
          pb: 3,
          width: 1,
          maxWidth: 540,
          display: 'none',
          overflow: 'hidden',
          position: 'relative',
          color: 'common.white',
          bgcolor: 'grey.900',
          pt: 'var(--layout-header-desktop-height)',
          borderRight: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.08)}`,
          '&::before': {
            inset: 0,
            content: "''",
            position: 'absolute',
            backgroundImage: [
              `linear-gradient(90deg, ${varAlpha(theme.vars.palette.common.whiteChannel, 0.04)} 1px, transparent 1px)`,
              `linear-gradient(0deg, ${varAlpha(theme.vars.palette.common.whiteChannel, 0.04)} 1px, transparent 1px)`,
            ].join(','),
            backgroundSize: '42px 42px',
            maskImage: 'linear-gradient(180deg, transparent, #000 14%, #000 82%, transparent)',
          },
          '&::after': {
            inset: 0,
            content: "''",
            position: 'absolute',
            background:
              'linear-gradient(140deg, rgba(242,184,27,0.22), transparent 38%), linear-gradient(320deg, rgba(4,210,242,0.16), transparent 44%)',
          },
          [theme.breakpoints.up(layoutQuery)]: {
            gap: 4,
            display: 'flex',
            alignItems: 'center',
            flexDirection: 'column',
            justifyContent: 'center',
          },
        }),
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
      {...other}
    >
      <Stack spacing={4} sx={{ zIndex: 1, width: 1, maxWidth: 430 }}>
        <Stack spacing={1.5}>
          <Typography
            variant="overline"
            sx={{ color: 'primary.light', letterSpacing: 0, fontWeight: 700 }}
          >
            {t('auth_panel.eyebrow')}
          </Typography>

          <Typography variant="h3" sx={{ maxWidth: 390 }}>
            {title ?? t('auth_panel.title')}
          </Typography>

          <Typography
            sx={(theme) => ({ color: varAlpha(theme.vars.palette.common.whiteChannel, 0.72) })}
          >
            {subtitle ?? t('auth_panel.subtitle')}
          </Typography>
        </Stack>

        <Box
          sx={(theme) => ({
            p: 2,
            borderRadius: 1,
            position: 'relative',
            border: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.12)}`,
            bgcolor: varAlpha(theme.vars.palette.common.whiteChannel, 0.06),
            boxShadow: `0 28px 80px ${varAlpha(theme.vars.palette.common.blackChannel, 0.32)}`,
            backdropFilter: 'blur(12px)',
          })}
        >
          <Stack spacing={2}>
            <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
              <Box
                component="img"
                alt=""
                src={`${CONFIG.assetsDir}/assets/icons/bitcoin/ic-bitcoin-lightning.svg`}
                sx={{ width: 44, height: 44 }}
              />

              <Box sx={{ minWidth: 0, flex: '1 1 auto' }}>
                <Typography variant="subtitle1">{t('auth_panel.preview_title')}</Typography>
                <Typography
                  variant="caption"
                  sx={(theme) => ({
                    color: varAlpha(theme.vars.palette.common.whiteChannel, 0.62),
                  })}
                >
                  {t('auth_panel.preview_caption')}
                </Typography>
              </Box>

              <Iconify
                icon="solar:lock-password-bold-duotone"
                width={28}
                sx={{ color: 'info.main' }}
              />
            </Stack>

            <Box
              sx={(theme) => ({
                gap: 1,
                display: 'grid',
                gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
                borderTop: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.1)}`,
                pt: 2,
              })}
            >
              {statusItems.map(([label, value]) => (
                <Box key={label} sx={{ minWidth: 0 }}>
                  <Typography
                    variant="caption"
                    sx={(theme) => ({
                      display: 'block',
                      color: varAlpha(theme.vars.palette.common.whiteChannel, 0.5),
                    })}
                  >
                    {t(label)}
                  </Typography>
                  <Typography variant="subtitle2" noWrap>
                    {t(value)}
                  </Typography>
                </Box>
              ))}
            </Box>
          </Stack>
        </Box>

        <Stack spacing={1.25}>
          {railItems.map((item) => (
            <Stack
              key={item.title}
              direction="row"
              spacing={1.5}
              sx={(theme) => ({
                p: 1.5,
                borderRadius: 1,
                alignItems: 'flex-start',
                border: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.1)}`,
                bgcolor: varAlpha(theme.vars.palette.common.whiteChannel, 0.04),
              })}
            >
              <Iconify icon={item.icon} width={24} sx={{ color: item.color, mt: 0.25 }} />

              <Box sx={{ minWidth: 0 }}>
                <Typography variant="subtitle2">{t(item.title)}</Typography>
                <Typography
                  variant="caption"
                  sx={(theme) => ({
                    color: varAlpha(theme.vars.palette.common.whiteChannel, 0.58),
                  })}
                >
                  {t(item.description)}
                </Typography>
              </Box>
            </Stack>
          ))}
        </Stack>
      </Stack>
    </Box>
  );
}
