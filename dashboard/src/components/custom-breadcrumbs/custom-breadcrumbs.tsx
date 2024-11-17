import Box from '@mui/material/Box';
import Link from '@mui/material/Link';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import Breadcrumbs from '@mui/material/Breadcrumbs';

import { BreadcrumbsLink } from './breadcrumb-link';

import type { CustomBreadcrumbsProps } from './types';

// ----------------------------------------------------------------------

export function CustomBreadcrumbs({ links, action, heading, moreLink, icon, activeLast, slotProps, sx, ...other }: CustomBreadcrumbsProps) {
  const lastLink = links[links.length - 1].name;

  const renderHeading = (
    <Typography variant="h4" sx={{ mb: 2, ...slotProps?.heading }}>
      {heading} {icon}
    </Typography>
  );

  const renderLinks = (
    <Breadcrumbs separator={<Separator />} sx={slotProps?.breadcrumbs} {...other}>
      {links.map((link, index) => (
        <BreadcrumbsLink key={link.name ?? index} link={link} activeLast={activeLast} disabled={link.name === lastLink} />
      ))}
    </Breadcrumbs>
  );

  const renderAction = <Box sx={{ flexShrink: 0, ...slotProps?.action }}> {action} </Box>;

  const renderMoreLink = (
    <Box component="ul">
      {moreLink?.map((href) => (
        <Box key={href} component="li" sx={{ display: 'flex' }}>
          <Link href={href} variant="body2" target="_blank" rel="noopener" sx={slotProps?.moreLink}>
            {href}
          </Link>
        </Box>
      ))}
    </Box>
  );

  return (
    <Stack spacing={2} sx={sx}>
      <Stack direction="row" alignItems="center">
        <Box sx={{ flexGrow: 1 }}>
          {heading && renderHeading}

          {!!links.length && renderLinks}
        </Box>

        {action && renderAction}
      </Stack>

      {!!moreLink && renderMoreLink}
    </Stack>
  );
}

// ----------------------------------------------------------------------

function Separator() {
  return (
    <Box
      component="span"
      sx={{
        width: 4,
        height: 4,
        borderRadius: '50%',
        bgcolor: 'text.disabled',
      }}
    />
  );
}
