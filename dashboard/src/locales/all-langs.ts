'use client';

// core (MUI)
import {
  frFR as frFRCore,
  deDE as deDECore,
  esES as esESCore,
  itIT as itITCore,
  ptPT as ptPTCore,
} from '@mui/material/locale';
// date pickers (MUI)
import {
  enUS as enUSDate,
  frFR as frFRDate,
  deDE as deDEDate,
  esES as esESDate,
  itIT as itITDate,
  ptPT as ptPTDate,
} from '@mui/x-date-pickers/locales';
// data grid (MUI)
import {
  enUS as enUSDataGrid,
  frFR as frFRDataGrid,
  deDE as deDEDataGrid,
  esES as esESDataGrid,
  itIT as itITDataGrid,
  ptPT as ptPTDataGrid,
} from '@mui/x-data-grid/locales';

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
    numberFormat: { code: 'fr-FR' },
    systemValue: {
      components: { ...frFRCore.components, ...frFRDate.components, ...frFRDataGrid.components },
    },
  },
  {
    value: 'de',
    label: 'Deutsch',
    countryCode: 'DE',
    adapterLocale: 'de',
    numberFormat: { code: 'de-DE' },
    systemValue: {
      components: { ...deDECore.components, ...deDEDate.components, ...deDEDataGrid.components },
    },
  },
  {
    value: 'es',
    label: 'Español',
    countryCode: 'ES',
    adapterLocale: 'es',
    numberFormat: { code: 'es-ES' },
    systemValue: {
      components: { ...esESCore.components, ...esESDate.components, ...esESDataGrid.components },
    },
  },
  {
    value: 'it',
    label: 'Italiano',
    countryCode: 'IT',
    adapterLocale: 'it',
    numberFormat: { code: 'it-IT' },
    systemValue: {
      components: { ...itITCore.components, ...itITDate.components, ...itITDataGrid.components },
    },
  },
  {
    value: 'pt',
    label: 'Português',
    countryCode: 'PT',
    adapterLocale: 'pt',
    numberFormat: { code: 'pt-PT' },
    systemValue: {
      components: { ...ptPTCore.components, ...ptPTDate.components, ...ptPTDataGrid.components },
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
