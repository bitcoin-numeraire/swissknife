'use client';

import type { LinkProps } from '@mui/material/Link';

import { mergeClasses } from 'minimal-shared/utils';

import Link from '@mui/material/Link';
import { styled, useColorScheme } from '@mui/material/styles';

import { RouterLink } from 'src/routes/components';

import { CONFIG } from 'src/global-config';
import { themeConfig } from 'src/theme/theme-config';

import { logoClasses } from './classes';

// ----------------------------------------------------------------------

export type LogoProps = LinkProps & {
  width?: number | string;
  height?: number | string;
  isSingle?: boolean;
  disabled?: boolean;
};

export function Logo({
  sx,
  width = 40,
  height = 40,
  disabled,
  className,
  href = '/',
  isSingle = false,
  ...other
}: LogoProps) {
  // Under MUI CSS variables, `theme.palette.mode` is not reactive — use the
  // color scheme hook so the logo follows the active light/dark scheme.
  const { mode, systemMode } = useColorScheme();
  const resolvedMode = (mode === 'system' ? systemMode : mode) ?? themeConfig.defaultMode;
  const isDark = resolvedMode === 'dark';

  const filename = isSingle ? 'logo_single' : 'logo_font';
  const variant = isDark ? filename : `${filename}_negative`;

  return (
    <LogoRoot
      component={RouterLink}
      href={href}
      aria-label="Logo"
      underline="none"
      className={mergeClasses([logoClasses.root, className])}
      sx={[
        {
          width,
          height,
          ...(disabled && { pointerEvents: 'none' }),
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
      {...other}
    >
      <img alt="Logo" src={`${CONFIG.assetsDir}/logo/${variant}.svg`} width="100%" height="100%" />
    </LogoRoot>
  );
}

// ----------------------------------------------------------------------

const LogoRoot = styled(Link)(() => ({
  flexShrink: 0,
  color: 'transparent',
  display: 'inline-flex',
  verticalAlign: 'middle',
}));
