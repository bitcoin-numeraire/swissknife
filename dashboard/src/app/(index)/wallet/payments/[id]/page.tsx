import { appTitle } from 'src/utils/format-string';

import { PaymentDetailsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payment Details'),
};

type Props = {
  params: {
    id: string;
  };
};

export default function PaymentDetailsPage({ params }: Props) {
  const { id } = params;

  return <PaymentDetailsView id={id} />;
}
