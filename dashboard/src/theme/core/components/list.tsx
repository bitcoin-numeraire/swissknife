import type { Theme, Components } from '@mui/material/styles';

// ----------------------------------------------------------------------

const MuiListItemIcon: Components<Theme>['MuiListItemIcon'] = {
  // ▼▼▼▼▼▼▼▼ 🎨 STYLE ▼▼▼▼▼▼▼▼
  styleOverrides: {
    root: ({ theme }) => ({
      color: 'inherit',
      minWidth: 'auto',
      marginRight: theme.spacing(2),
    }),
  },
};

const MuiListItemAvatar: Components<Theme>['MuiListItemAvatar'] = {
  // ▼▼▼▼▼▼▼▼ 🎨 STYLE ▼▼▼▼▼▼▼▼
  styleOverrides: {
    root: ({ theme }) => ({
      minWidth: 'auto',
      marginRight: theme.spacing(2),
    }),
  },
};

const MuiListItemText: Components<Theme>['MuiListItemText'] = {
  // ▼▼▼▼▼▼▼▼ ⚙️ PROPS ▼▼▼▼▼▼▼▼
  defaultProps: {
    slotProps: {
      primary: { sx: { typography: 'subtitle2' } },
    },
  },
  // ▼▼▼▼▼▼▼▼ 🎨 STYLE ▼▼▼▼▼▼▼▼
  styleOverrides: {
    root: { margin: 0 },
    multiline: { margin: 0 },
  },
};

/* **********************************************************************
 * 🚀 Export
 * **********************************************************************/
export const list: Components<Theme> = {
  MuiListItemIcon,
  MuiListItemText,
  MuiListItemAvatar,
};
