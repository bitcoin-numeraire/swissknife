import type { StackProps } from '@mui/material/Stack';

import Stack from '@mui/material/Stack';
import { useTheme } from '@mui/material/styles';
import Typography from '@mui/material/Typography';

// ----------------------------------------------------------------------

type Props = StackProps & {
  title?: string;
  description?: string;
  img?: React.ReactNode;
  action?: React.ReactNode;
};

export function Welcome({ title, description, action, img, ...other }: Props) {
  const theme = useTheme();

  return (
    <Stack
      flexDirection={{ xs: 'column', md: 'row' }}
      sx={{
        height: { md: 1 },
        borderRadius: 2,
        position: 'relative',
        backgroundColor: 'background.paper',
      }}
      {...other}
    >
      <Stack
        flexGrow={1}
        justifyContent="center"
        alignItems={{ xs: 'center', md: 'flex-start' }}
        sx={{
          p: {
            xs: theme.spacing(5, 3, 0, 3),
            md: theme.spacing(5),
          },
          textAlign: { xs: 'center', md: 'left' },
        }}
      >
        <Typography variant="h3" sx={{ mb: 2 }}>
          {title}
        </Typography>

        <Typography sx={{ mb: 3 }}>{description}</Typography>

        {action && action}
      </Stack>

      {img && (
        <Stack
          component="span"
          justifyContent="center"
          sx={{
            p: { xs: 5, md: 3 },
            maxWidth: 480,
            mx: 'auto',
          }}
        >
          {img}
        </Stack>
      )}
    </Stack>
  );
}
