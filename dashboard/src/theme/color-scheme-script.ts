'use client';

import { getInitColorSchemeScript as _getInitColorSchemeScript } from '@mui/material/styles';

import { defaultSettings } from 'src/components/settings';

// ----------------------------------------------------------------------

export const schemeConfig = {
  modeStorageKey: 'theme-mode',
  defaultMode: defaultSettings.colorScheme,
};

export const getInitColorSchemeScript = _getInitColorSchemeScript(schemeConfig);
