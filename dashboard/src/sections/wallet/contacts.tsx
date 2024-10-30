import type { Contact } from 'src/lib/swissknife';
import type { CardProps } from '@mui/material/Card';
import type { IFiatPrices } from 'src/types/bitcoin';

import { mutate } from 'swr';
import { useState } from 'react';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Avatar from '@mui/material/Avatar';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';
import CardHeader from '@mui/material/CardHeader';
import ListItemText from '@mui/material/ListItemText';

import { paths } from 'src/routes/paths';

import { useBoolean } from 'src/hooks/use-boolean';

import { fFromNow } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';

import { Iconify } from 'src/components/iconify';
import { ConfirmPaymentDialog } from 'src/components/transactions';

// ----------------------------------------------------------------------

interface Props extends CardProps {
  title?: string;
  list: Contact[];
  fiatPrices: IFiatPrices;
}

export function Contacts({ title, fiatPrices, list, ...other }: Props) {
  const { t } = useTranslate();
  const [input, setInput] = useState('');
  const confirm = useBoolean();

  const handleClick = (ln_address: string) => {
    setInput(ln_address);
    confirm.onTrue();
  };

  const handleClose = () => {
    setInput('');
    confirm.onFalse();
  };

  return (
    <Card {...other}>
      <CardHeader
        title={title}
        subheader={t('wallet_contacts.subheader', { count: list.length })}
        action={
          <Button
            href={paths.wallet.contacts}
            size="small"
            color="inherit"
            endIcon={<Iconify icon="eva:arrow-ios-forward-fill" width={18} sx={{ ml: -0.5 }} />}
          >
            {t('view_all')}
          </Button>
        }
      />

      <Stack spacing={3} sx={{ p: 3 }}>
        {list.map((contact) => (
          <Stack direction="row" alignItems="center" key={contact.ln_address}>
            <Avatar alt={contact.ln_address} sx={{ width: 48, height: 48, mr: 2 }}>
              {contact.ln_address.charAt(0).toUpperCase()}
            </Avatar>

            <ListItemText primary={contact.ln_address} secondary={fFromNow(contact.contact_since)} />

            <Tooltip title={t('wallet_contacts.quick_transfer')}>
              <IconButton onClick={() => handleClick(contact.ln_address)}>
                <Iconify icon="eva:diagonal-arrow-right-up-fill" />
              </IconButton>
            </Tooltip>
          </Stack>
        ))}
      </Stack>

      <ConfirmPaymentDialog
        input={input}
        open={confirm.value}
        onClose={handleClose}
        onSuccess={() => mutate(endpointKeys.userWallet.get)}
        fiatPrices={fiatPrices}
      />
    </Card>
  );
}
