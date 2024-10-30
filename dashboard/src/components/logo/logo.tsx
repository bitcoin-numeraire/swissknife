'use client';

import type { BoxProps } from '@mui/material/Box';

import { forwardRef } from 'react';

import Box from '@mui/material/Box';
import NoSsr from '@mui/material/NoSsr';
import { useTheme } from '@mui/material/styles';

import { RouterLink } from 'src/routes/components';

import { CONFIG } from 'src/config-global';

import { logoClasses } from './classes';

// ----------------------------------------------------------------------

export type LogoProps = BoxProps & {
  href?: string;
  disableLink?: boolean;
  type?: 'single' | 'full' | 'font';
};

export const Logo = forwardRef<HTMLDivElement, LogoProps>(
  ({ width = 40, height = 40, disableLink = false, className, href = '/', type = 'single', sx, ...other }, ref) => {
    const theme = useTheme();

    let filename = 'logo_single';
    if (type === 'full') {
      filename = 'logo';
    } else if (type === 'font') {
      filename = 'logo_font';
    }

    const logo = (
      <Box
        alt="logo"
        component="img"
        src={`${CONFIG.site.basePath}/logo/${theme.palette.mode === 'dark' ? filename : `${filename}_negative`}.svg`}
        width={width}
        height={height}
      />
    );

    return (
      <NoSsr
        fallback={
          <Box
            width={width}
            height={height}
            className={logoClasses.root.concat(className ? ` ${className}` : '')}
            sx={{ flexShrink: 0, display: 'inline-flex', verticalAlign: 'middle', ...sx }}
          />
        }
      >
        <Box
          ref={ref}
          component={RouterLink}
          href={href}
          width={width}
          height={height}
          className={logoClasses.root.concat(className ? ` ${className}` : '')}
          aria-label="logo"
          sx={{
            flexShrink: 0,
            display: 'inline-flex',
            verticalAlign: 'middle',
            ...(disableLink && { pointerEvents: 'none' }),
            ...sx,
          }}
          {...other}
        >
          {logo}
        </Box>
      </NoSsr>
    );
  }
);
