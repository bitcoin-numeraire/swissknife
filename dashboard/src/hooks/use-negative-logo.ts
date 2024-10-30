import { useTheme } from '@mui/material';

export function useNegativeLogo(filename: string): string {
  const theme = useTheme();

  return theme.palette.mode === 'dark' ? filename : `${filename}_negative`;
}
