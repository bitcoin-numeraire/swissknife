import type { Theme, Components } from '@mui/material/styles';

import { varAlpha } from 'minimal-shared/utils';

// ----------------------------------------------------------------------

const MuiLink: Components<Theme>['MuiLink'] = {
  // ▼▼▼▼▼▼▼▼ ⚙️ PROPS ▼▼▼▼▼▼▼▼
  defaultProps: {
    underline: 'hover',
  },
  // ▼▼▼▼▼▼▼▼ 🎨 STYLE ▼▼▼▼▼▼▼▼
  styleOverrides: {
    root: {
      '--Link-underlineColor': varAlpha('currentColor', 0.4),
    },
  },
};

/* **********************************************************************
 * 🚀 Export
 * **********************************************************************/
export const link: Components<Theme> = {
  MuiLink,
};
