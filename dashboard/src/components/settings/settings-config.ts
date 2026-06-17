import type { SettingsState } from './types';

import { CONFIG } from 'src/global-config';
import { themeConfig } from 'src/theme/theme-config';

// ----------------------------------------------------------------------

export const SETTINGS_STORAGE_KEY: string = 'app-settings';
export const ONBOARDING_COMPLETE_STORAGE_KEY = 'onboarding-complete';

export const defaultSettings: SettingsState = {
  mode: themeConfig.defaultMode,
  direction: themeConfig.direction,
  contrast: 'default',
  compactLayout: true,
  currency: 'USD',
  version: CONFIG.appVersion,
};
