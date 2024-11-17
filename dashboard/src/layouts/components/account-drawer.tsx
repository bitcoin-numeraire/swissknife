'use client';

import type { IconButtonProps } from '@mui/material/IconButton';

import { useCallback } from 'react';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Drawer from '@mui/material/Drawer';
import MenuItem from '@mui/material/MenuItem';
import { useTheme } from '@mui/material/styles';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { useRouter } from 'src/routes/hooks';

import { useBoolean } from 'src/hooks/use-boolean';

import { CONFIG } from 'src/config-global';
import { varAlpha } from 'src/theme/styles';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { AnimateAvatar } from 'src/components/animate';

import { useAuthContext } from 'src/auth/hooks';

import { AccountButton } from './account-button';
import { SignOutButton } from './sign-out-button';

// ----------------------------------------------------------------------

export type AccountDrawerProps = IconButtonProps & {
  data?: {
    label: string;
    href: string;
    icon?: React.ReactNode;
    info?: React.ReactNode;
    target?: string;
  }[];
};

export function AccountDrawer({ data = [], sx, ...other }: AccountDrawerProps) {
  const theme = useTheme();

  const router = useRouter();

  const { user } = useAuthContext();

  const open = useBoolean(false);

  const handleClickItem = useCallback(
    (path: string, target: string = '_self') => {
      open.onFalse();
      if (target === '_blank') {
        window.open(path, target);
      } else {
        router.push(path);
      }
    },
    [open, router]
  );

  const renderAvatar = (
    <AnimateAvatar
      width={96}
      slotProps={{
        avatar: { src: user?.photoURL, alt: user?.displayName },
        overlay: {
          border: 2,
          spacing: 3,
          color: `linear-gradient(135deg, ${varAlpha(theme.vars.palette.primary.mainChannel, 0)} 25%, ${theme.vars.palette.primary.main} 100%)`,
        },
      }}
    >
      {user?.displayName?.charAt(0).toUpperCase()}
    </AnimateAvatar>
  );

  return (
    <>
      <AccountButton open={open.value} onClick={open.onTrue} photoURL={user?.photoURL} displayName={user?.displayName} sx={sx} {...other} />

      <Drawer
        open={open.value}
        onClose={open.onFalse}
        anchor="right"
        slotProps={{ backdrop: { invisible: true } }}
        PaperProps={{ sx: { width: 320 } }}
      >
        <IconButton onClick={open.onFalse} sx={{ top: 12, left: 12, zIndex: 9, position: 'absolute' }}>
          <Iconify icon="mingcute:close-line" />
        </IconButton>

        <Scrollbar>
          <Stack alignItems="center" sx={{ pt: 8, pb: 3 }}>
            {renderAvatar}

            <Typography variant="subtitle1" noWrap sx={{ mt: 2 }}>
              {user?.displayName}
            </Typography>

            <Typography variant="body2" sx={{ color: 'text.secondary', mt: 0.5 }} noWrap>
              {user?.email}
            </Typography>
          </Stack>

          <Stack
            sx={{
              py: 3,
              px: 2.5,
              borderTop: `dashed 1px ${theme.vars.palette.divider}`,
              borderBottom: `dashed 1px ${theme.vars.palette.divider}`,
            }}
          >
            {data.map((option) => (
              <MenuItem
                key={option.label}
                onClick={() => handleClickItem(option.label === 'Home' ? '/' : option.href, option.target)}
                sx={{
                  py: 1,
                  color: 'text.secondary',
                  '& svg': { width: 24, height: 24 },
                  '&:hover': { color: 'text.primary' },
                }}
              >
                {option.icon}

                <Box component="span" sx={{ ml: 2 }}>
                  {option.label === 'Home' ? 'Home' : option.label}
                </Box>

                {option.info && (
                  <Label color="error" sx={{ ml: 1 }}>
                    {option.info}
                  </Label>
                )}
              </MenuItem>
            ))}
          </Stack>

          <Box sx={{ px: 2.5, py: 3 }}>
            <Stack alignItems="flex-start" sx={{ position: 'relative' }}>
              <Box component="span" sx={{ mb: 2, color: 'text.secondary', typography: 'body2' }}>
                Version: {CONFIG.site.version}
              </Box>

              <Box component="span" sx={{ typography: 'body2', color: 'text.secondary' }}>
                Built with <Iconify icon="solar:heart-bold" sx={{ color: 'red' }} /> from Switzerland
              </Box>
            </Stack>
          </Box>
        </Scrollbar>
        <Box sx={{ p: 2.5 }}>
          <SignOutButton onClose={open.onFalse} />
        </Box>
      </Drawer>
    </>
  );
}
