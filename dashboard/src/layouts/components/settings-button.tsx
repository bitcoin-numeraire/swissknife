import type { IconButtonProps } from '@mui/material/IconButton';

import Badge from '@mui/material/Badge';
import SvgIcon from '@mui/material/SvgIcon';
import IconButton from '@mui/material/IconButton';

import { useSettingsContext } from 'src/components/settings';

// ----------------------------------------------------------------------

export function SettingsButton({ sx, ...other }: IconButtonProps) {
  const settings = useSettingsContext();

  return (
    <IconButton
      aria-label="Settings button"
      onClick={settings.onToggleDrawer}
      sx={[{ p: 0, width: 40, height: 40 }, ...(Array.isArray(sx) ? sx : [sx])]}
      {...other}
    >
      <Badge color="error" variant="dot" invisible={!settings.canReset}>
        <SvgIcon>
          {/* https://yesicon.app/solar/pallete-2-bold-duotone */}
          <path
            fill="currentColor"
            fillRule="evenodd"
            d="M10.847 21.934C5.867 21.362 2 17.133 2 12C2 6.477 6.477 2 12 2s10 4.477 10 10c0 5.157-3.283 4.733-6.086 4.37c-1.618-.209-3.075-.397-3.652.518c-.395.626.032 1.406.555 1.929a1.673 1.673 0 0 1 0 2.366c-.523.523-1.235.836-1.97.751"
            clipRule="evenodd"
            opacity="0.5"
          />
          <path
            fill="currentColor"
            d="M11.085 7a1.5 1.5 0 1 1-3 0a1.5 1.5 0 0 1 3 0M6.5 13a1.5 1.5 0 1 0 0-3a1.5 1.5 0 0 0 0 3m11 0a1.5 1.5 0 1 0 0-3a1.5 1.5 0 0 0 0 3m-3-4.5a1.5 1.5 0 1 0 0-3a1.5 1.5 0 0 0 0 3"
          />
        </SvgIcon>
      </Badge>
    </IconButton>
  );
}
