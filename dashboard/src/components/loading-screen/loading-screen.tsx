'use client';

import type { Theme, SxProps } from '@mui/material/styles';
import type { LinearProgressProps } from '@mui/material/LinearProgress';

import Portal from '@mui/material/Portal';
import { styled } from '@mui/material/styles';
import LinearProgress from '@mui/material/LinearProgress';

// ----------------------------------------------------------------------

export type LoadingScreenProps = React.ComponentProps<'div'> & {
  portal?: boolean;
  sx?: SxProps<Theme>;
  slots?: {
    progress?: React.ReactNode;
  };
  slotsProps?: {
    progress?: LinearProgressProps;
  };
};

export function LoadingScreen({ portal, slots, slotsProps, sx, ...other }: LoadingScreenProps) {
  const renderContent = (
    <LoadingContent sx={sx} {...other}>
      {slots?.progress ?? (
        <LinearProgress
          color="inherit"
          sx={[
            { width: 1, maxWidth: 360 },
            ...(Array.isArray(slotsProps?.progress?.sx)
              ? slotsProps.progress.sx
              : [slotsProps?.progress?.sx]),
          ]}
          {...slotsProps?.progress}
        />
      )}
    </LoadingContent>
  );

  if (portal) {
    return <Portal>{renderContent}</Portal>;
  }

  return renderContent;
}

// ----------------------------------------------------------------------

const LoadingContent = styled('div')(({ theme }) => ({
  flexGrow: 1,
  width: '100%',
  display: 'flex',
  minHeight: '100%',
  alignItems: 'center',
  justifyContent: 'center',
  paddingLeft: theme.spacing(5),
  paddingRight: theme.spacing(5),
}));
