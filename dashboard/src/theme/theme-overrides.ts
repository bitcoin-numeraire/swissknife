'use client';

import { createPaletteChannel } from 'minimal-shared/utils';

import type { ThemeOptions } from './types';

// ----------------------------------------------------------------------

export const themeOverrides: ThemeOptions = {
  colorSchemes: {
    light: {
      palette: {
        primary: createPaletteChannel({
          lighter: '#FEF7D1',
          light: '#FBDE75',
          main: '#F2B81B',
          dark: '#AE790D',
          darker: '#744905',
          contrastText: '#FFFFFF',
        }),
      },
    },
    dark: {
      palette: {
        primary: createPaletteChannel({
          lighter: '#FEF7D1',
          light: '#FBDE75',
          main: '#F2B81B',
          dark: '#AE790D',
          darker: '#744905',
          contrastText: '#FFFFFF',
        }),
      },
    },
  },
};
