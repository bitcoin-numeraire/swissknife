'use client';

import type { Contact } from 'src/lib/swissknife';
import type { IFiatPrices } from 'src/types/bitcoin';

import { useState } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Button from '@mui/material/Button';
import Avatar from '@mui/material/Avatar';
import ListItemText from '@mui/material/ListItemText';

import { fFromNow } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';

import { SendMoneyDrawer } from 'src/sections/wallet/money-drawers';

// ----------------------------------------------------------------------

type Props = {
  data: Contact[];
  fiatPrices: IFiatPrices;
};

export function ContactList({ data, fiatPrices }: Props) {
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
    <Box
      sx={{
        gap: 3,
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(min(100%, 380px), 1fr))',
      }}
    >
      {data.map((contact) => (
        <Card
          key={contact.ln_address}
          sx={{ display: 'flex', alignItems: 'center', p: (theme) => theme.spacing(3, 2, 3, 3) }}
        >
          <Avatar alt={contact.ln_address} sx={{ width: 48, height: 48, mr: 2 }}>
            {contact.ln_address.charAt(0).toUpperCase()}
          </Avatar>

          <ListItemText
            primary={contact.ln_address}
            secondary={fFromNow(contact.contact_since)}
            sx={{ minWidth: 0, flex: 1 }}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'subtitle2' } },
              secondary: {
                noWrap: true,
                sx: { mt: 0.5, typography: 'caption' },
              },
            }}
          />

          <Button
            size="small"
            variant="outlined"
            onClick={() => handleClick(contact.ln_address)}
            startIcon={
              <Iconify width={18} icon="eva:diagonal-arrow-right-up-fill" sx={{ mr: -0.75 }} />
            }
            sx={{ flexShrink: 0, ml: 1.5 }}
          >
            {t('send')}
          </Button>
        </Card>
      ))}

      <SendMoneyDrawer
        contacts={data}
        initialInput={input}
        open={confirm.value}
        onClose={handleClose}
        fiatPrices={fiatPrices}
      />
    </Box>
  );
}
