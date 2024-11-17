'use client';

// core (MUI)
import { frFR as frFRCore } from '@mui/material/locale';
// date pickers (MUI)
import { enUS as enUSDate, frFR as frFRDate } from '@mui/x-date-pickers/locales';
// data grid (MUI)
import { enUS as enUSDataGrid, frFR as frFRDataGrid } from '@mui/x-data-grid/locales';

// ----------------------------------------------------------------------

export const allLangs = [
  {
    value: 'en',
    label: 'English',
    countryCode: 'GB',
    adapterLocale: 'en',
    numberFormat: { code: 'en-US' },
    systemValue: {
      components: { ...enUSDate.components, ...enUSDataGrid.components },
    },
  },
  {
    value: 'fr',
    label: 'Français',
    countryCode: 'FR',
    adapterLocale: 'fr',
    numberFormat: { code: 'fr-Fr' },
    systemValue: {
      components: { ...frFRCore.components, ...frFRDate.components, ...frFRDataGrid.components },
    },
  },
];

/**
 * Country code:
 * https://flagcdn.com/en/codes.json
 *
 * Number format code:
 * https://gist.github.com/raushankrjha/d1c7e35cf87e69aa8b4208a8171a8416
 */
