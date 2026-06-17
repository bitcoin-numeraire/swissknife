import type { Theme, SxProps } from '@mui/material/styles';
import type { CurrencyValue } from 'src/types/currency';
import type { ThemeConfig } from 'src/theme/theme-config';
import type { ThemeColorPreset } from 'src/theme/with-settings';

// ----------------------------------------------------------------------

export type SettingsState = {
  version: string;
  mode: ThemeConfig['defaultMode'];
  contrast: 'default' | 'high';
  compactLayout: boolean;
  currency: CurrencyValue;
  // Optional: omitted from defaultSettings so the settings drawer hides these controls
  // (the dashboard intentionally exposes only dark mode, contrast, RTL and compact).
  fontSize?: number;
  fontFamily?: string;
  navColor?: 'integrate' | 'apparent';
  navLayout?: 'vertical' | 'horizontal' | 'mini';
  primaryColor?: ThemeColorPreset;
};

export type SettingsContextValue = {
  state: SettingsState;
  canReset: boolean;
  onReset: () => void;
  setState: (updateValue: Partial<SettingsState>) => void;
  setField: (name: keyof SettingsState, updateValue: SettingsState[keyof SettingsState]) => void;
  // Drawer
  openDrawer: boolean;
  onCloseDrawer: () => void;
  onToggleDrawer: () => void;
};

export type SettingsProviderProps = {
  cookieSettings?: SettingsState;
  defaultSettings: SettingsState;
  children: React.ReactNode;
  storageKey?: string;
};

export type SettingsDrawerProps = {
  sx?: SxProps<Theme>;
  defaultSettings: SettingsState;
};
