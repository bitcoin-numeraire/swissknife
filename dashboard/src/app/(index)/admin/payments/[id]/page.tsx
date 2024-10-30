import { appTitle } from 'src/utils/format-string';

import { AdminPaymentDetailsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payment Details'),
};

type Props = {
  params: {
    id: string;
  };
};

export default function AdminPaymentDetailsPage({ params }: Props) {
  const { id } = params;

  return <AdminPaymentDetailsView id={id} />;
}
