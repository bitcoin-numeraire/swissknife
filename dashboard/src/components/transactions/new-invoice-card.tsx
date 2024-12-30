import type { CardProps } from '@mui/material';

import Box from '@mui/material/Box';
import { Card, CardHeader } from '@mui/material';

import { NewInvoiceForm } from './new-invoice-form';

import type { NewInvoiceFormProps } from './new-invoice-form';

// ----------------------------------------------------------------------

type Props = CardProps &
  NewInvoiceFormProps & {
    subheader?: string;
  };

export function NewInvoiceCard({
  onSuccess,
  title,
  subheader,
  lnAddress,
  fiatPrices,
  sx,
  ...other
}: Props) {
  return (
    <Card {...other}>
      <CardHeader title={title} subheader={subheader} />

      <Box sx={{ p: 3 }}>
        <NewInvoiceForm fiatPrices={fiatPrices} lnAddress={lnAddress} onSuccess={onSuccess} />
      </Box>
    </Card>
  );
}
