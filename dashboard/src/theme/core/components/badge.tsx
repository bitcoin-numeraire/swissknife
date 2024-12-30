import type { Theme, Components } from '@mui/material/styles';

import { badgeClasses } from '@mui/material/Badge';

// ----------------------------------------------------------------------

/**
 * TypeScript (type definition and extension)
 * @to {@link file://./../../extend-theme-types.d.ts}
 */

export type BadgeExtendVariant = {
  always: true;
  busy: true;
  online: true;
  offline: true;
  invisible: true;
};

// ----------------------------------------------------------------------

const baseStyles = (theme: Theme) => ({
  width: 10,
  zIndex: 9,
  bottom: 0,
  padding: 0,
  height: 10,
  top: 'auto',
  minWidth: 'auto',
  '&::before, &::after': {
    content: "''",
    borderRadius: 1,
    backgroundColor: theme.vars.palette.common.white,
  },
  [`&.${badgeClasses.invisible}`]: {
    transform: 'unset',
  },
});

const MuiBadge: Components<Theme>['MuiBadge'] = {
  /** **************************************
   * STYLE
   *************************************** */
  styleOverrides: {
    badge: ({ ownerState, theme }) => ({
      /**
       * @variant online
       */
      ...(ownerState.variant === 'online' && {
        ...baseStyles(theme),
        backgroundColor: theme.vars.palette.success.main,
      }),
      /**
       * @variant always
       */
      ...(ownerState.variant === 'always' && {
        ...baseStyles(theme),
        backgroundColor: theme.vars.palette.warning.main,
        '&::before': { width: 2, height: 4, transform: 'translate(1px, -1px)' },
        '&::after': { width: 2, height: 4, transform: 'translate(0, 1px) rotate(125deg)' },
      }),
      /**
       * @variant busy
       */
      ...(ownerState.variant === 'busy' && {
        ...baseStyles(theme),
        backgroundColor: theme.vars.palette.error.main,
        '&::before': { width: 6, height: 2 },
      }),
      /**
       * @variant offline
       */
      ...(ownerState.variant === 'offline' && {
        ...baseStyles(theme),
        backgroundColor: theme.vars.palette.text.disabled,
        '&::before': { width: 6, height: 6, borderRadius: '50%' },
      }),
      /**
       * @variant invisible
       */
      ...(ownerState.variant === 'invisible' && {
        display: 'none',
      }),
    }),
    dot: { borderRadius: '50%' },
  },
};

// ----------------------------------------------------------------------

export const badge = { MuiBadge };
