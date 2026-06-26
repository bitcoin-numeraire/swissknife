import type { SettingsState } from './types';

import { CONFIG } from 'src/global-config';
import { themeConfig } from 'src/theme/theme-config';

// ----------------------------------------------------------------------

export const SETTINGS_STORAGE_KEY: string = 'app-settings';
export const ONBOARDING_COMPLETE_STORAGE_KEY = 'onboarding-complete';

export const defaultSettings: SettingsState = {
  mode: themeConfig.defaultMode,
  contrast: 'default',
  compactLayout: true,
  currency: 'USD',
  displayUnit: 'bip177',
  hideBalances: false,
  defaultAddressType: 'p2tr',
  version: CONFIG.appVersion,
};
