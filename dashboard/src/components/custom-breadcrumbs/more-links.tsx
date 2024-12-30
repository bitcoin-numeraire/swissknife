import Link from '@mui/material/Link';
import { styled } from '@mui/material/styles';

// ----------------------------------------------------------------------

export type MoreLinksProps = React.ComponentProps<typeof MoreLinksRoot> & {
  links?: string[];
};

export function MoreLinks({ links, sx, ...other }: MoreLinksProps) {
  return (
    <MoreLinksRoot sx={sx} {...other}>
      {links?.map((href) => (
        <li key={href}>
          <Link href={href} variant="body2" target="_blank" rel="noopener">
            {href}
          </Link>
        </li>
      ))}
    </MoreLinksRoot>
  );
}

// ----------------------------------------------------------------------

const MoreLinksRoot = styled('ul')(() => ({
  display: 'flex',
  flexDirection: 'column',
  '& > li': { display: 'flex' },
}));
