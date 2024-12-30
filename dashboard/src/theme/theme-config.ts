import type { CommonColors } from '@mui/material/styles/createPalette';

import type { PaletteColorNoChannels } from './core/palette';
import type { ThemeDirection, ThemeColorScheme, ThemeCssVariables } from './types';

// ----------------------------------------------------------------------

type ThemeConfig = {
  classesPrefix: string;
  modeStorageKey: string;
  direction: ThemeDirection;
  defaultMode: ThemeColorScheme;
  cssVariables: ThemeCssVariables;
  fontFamily: Record<'primary' | 'secondary', string>;
  palette: Record<
    'primary' | 'secondary' | 'info' | 'success' | 'warning' | 'error',
    PaletteColorNoChannels
  > & {
    common: Pick<CommonColors, 'black' | 'white'>;
    grey: Record<
      '50' | '100' | '200' | '300' | '400' | '500' | '600' | '700' | '800' | '900',
      string
    >;
  };
};

export const themeConfig: ThemeConfig = {
  /** **************************************
   * Base
   *************************************** */
  direction: 'ltr',
  defaultMode: 'dark',
  modeStorageKey: 'theme-mode',
  classesPrefix: 'numeraire',
  /** **************************************
   * Typography
   *************************************** */
  fontFamily: {
    primary: 'Inter Variable',
    secondary: 'Public Sans Variable',
  },
  /** **************************************
   * Palette
   *************************************** */
  palette: {
    primary: {
      lighter: '#FEF7D1',
      light: '#FBDE75',
      main: '#F2B81B',
      dark: '#AE790D',
      darker: '#744905',
      contrastText: '#FFFFFF',
    },
    secondary: {
      lighter: '#FCF1D2',
      light: '#EFC677',
      main: '#CC8221',
      dark: '#924F10',
      darker: '#612A06',
      contrastText: '#FFFFFF',
    },
    info: {
      lighter: '#CCFEF5',
      light: '#66FBF7',
      main: '#04D2F2',
      dark: '#027BAE',
      darker: '#004074',
      contrastText: '#FFFFFF',
    },
    success: {
      lighter: '#EAFBDB',
      light: '#ACE890',
      main: '#55B542',
      dark: '#228221',
      darker: '#0C5617',
      contrastText: '#ffffff',
    },
    warning: {
      lighter: '#FFFBD6',
      light: '#FFF183',
      main: '#FFE332',
      dark: '#B79D19',
      darker: '#7A6409',
      contrastText: '#1C252E',
    },
    error: {
      lighter: '#FFE9D6',
      light: '#FFAB83',
      main: '#FF5432',
      dark: '#B71B19',
      darker: '#7A0919',
      contrastText: '#FFFFFF',
    },
    grey: {
      '50': '#f9f9f9',
      '100': '#f2f2f2',
      '200': '#eaeaea',
      '300': '#dadada',
      '400': '#b6b6b6',
      '500': '#979797',
      '600': '#6e6e6e',
      '700': '#3C3C3C',
      '800': '#282828',
      '900': '#1E1E1E',
    },
    common: {
      black: '#000000',
      white: '#FFFFFF',
    },
  },
  /** **************************************
   * Css variables
   *************************************** */
  cssVariables: {
    cssVarPrefix: '',
    colorSchemeSelector: 'data-color-scheme',
  },
};
