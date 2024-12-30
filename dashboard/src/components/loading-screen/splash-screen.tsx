'use client';

import type { Theme, SxProps } from '@mui/material/styles';

import Portal from '@mui/material/Portal';
import { styled } from '@mui/material/styles';

import { AnimateLogoZoom } from 'src/components/animate';

// ----------------------------------------------------------------------

export type SplashScreenProps = React.ComponentProps<'div'> & {
  portal?: boolean;
  sx?: SxProps<Theme>;
};

export function SplashScreen({ portal = true, sx, ...other }: SplashScreenProps) {
  const content = (
    <div style={{ overflow: 'hidden' }}>
      <LoadingContent sx={sx} {...other}>
        <AnimateLogoZoom />
      </LoadingContent>
    </div>
  );

  if (portal) {
    return <Portal>{content}</Portal>;
  }

  return content;
}

// ----------------------------------------------------------------------

const LoadingContent = styled('div')(({ theme }) => ({
  right: 0,
  bottom: 0,
  zIndex: 9998,
  width: '100%',
  height: '100%',
  display: 'flex',
  position: 'fixed',
  alignItems: 'center',
  justifyContent: 'center',
  backgroundColor: theme.vars.palette.background.default,
}));
