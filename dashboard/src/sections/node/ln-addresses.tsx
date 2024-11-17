import type { CardProps } from '@mui/material/Card';
import type { ListAddressesResponse } from 'src/lib/swissknife';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Avatar from '@mui/material/Avatar';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';
import CardHeader from '@mui/material/CardHeader';
import ListItemText from '@mui/material/ListItemText';

import { paths } from 'src/routes/paths';

import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';

// ----------------------------------------------------------------------

interface Props extends CardProps {
  subheader?: string;
  list: ListAddressesResponse;
}

export function LnAddresses({ subheader, list, ...other }: Props) {
  const { t } = useTranslate();

  return (
    <Card {...other}>
      <CardHeader
        title={t('latest_ln_addresses.card_title')}
        subheader={subheader}
        action={
          <Button
            size="small"
            color="inherit"
            href={paths.admin.lnAddresses}
            endIcon={<Iconify icon="eva:arrow-ios-forward-fill" width={18} sx={{ ml: -0.5 }} />}
          >
            {t('view_all')}
          </Button>
        }
      />

      <Stack spacing={3} sx={{ p: 3 }}>
        {list.map((address) => (
          <Stack direction="row" alignItems="center" key={address.id}>
            <Avatar alt={address.username} sx={{ width: 48, height: 48, mr: 2 }}>
              {address.username.charAt(0).toUpperCase()}
            </Avatar>

            <ListItemText primary={address.username} secondary={truncateText(address.wallet_id, 15)} />

            <Tooltip title={t('details')}>
              <IconButton href={paths.admin.lnAddress(address.id)}>
                <Iconify icon="eva:diagonal-arrow-right-up-fill" />
              </IconButton>
            </Tooltip>
          </Stack>
        ))}
      </Stack>
    </Card>
  );
}
