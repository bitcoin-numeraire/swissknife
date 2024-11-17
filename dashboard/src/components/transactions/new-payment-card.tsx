import type { CardProps } from '@mui/material';
import type { IFiatPrices } from 'src/types/bitcoin';

import Box from '@mui/material/Box';
import { Card, CardHeader } from '@mui/material';

import { NewPaymentForm } from './new-payment-form';

import type { NewPaymentFormProps } from './new-payment-form';

// ----------------------------------------------------------------------

interface Props extends CardProps, NewPaymentFormProps {
  title?: string;
  subheader?: string;
  fiatPrices: IFiatPrices;
  onSuccess: VoidFunction;
}

export function NewPaymentCard({ title, subheader, sx, fiatPrices, onSuccess, ...other }: Props) {
  return (
    <Card {...other}>
      <CardHeader title={title} subheader={subheader} />

      <Box sx={{ p: 3 }}>
        <NewPaymentForm onSuccess={onSuccess} fiatPrices={fiatPrices} {...other} />
      </Box>
    </Card>
  );
}
