import { themeConfig } from 'src/theme/theme-config';

import type { SettingsState } from './types';

// ----------------------------------------------------------------------

export const SETTINGS_STORAGE_KEY: string = 'app-settings';

export const defaultSettings: SettingsState = {
  colorScheme: themeConfig.defaultMode,
  contrast: 'default',
  compactLayout: true,
  fontSize: 16,
  currency: 'USD',
  fontFamily: themeConfig.fontFamily.primary,
};
